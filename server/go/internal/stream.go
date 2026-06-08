package internal

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"image"
	"io"
	"log"
	"net/http"
	"sync"
	"sync/atomic"
	"time"

	"github.com/gorilla/websocket"
	"github.com/pion/webrtc/v3"
	"github.com/pixiv/go-libjpeg/jpeg"
)

type encodeResult struct {
	data []byte
}

var pipeline = make(chan chan encodeResult, 128)

var firstFrameGo = true
var firstFrameForwardedGo = true

var droppedFrames uint64
var totalFrames uint64

var pixelPool = sync.Pool{
	New: func() interface{} {
		return nil
	},
}

var bufferPool = sync.Pool{
	New: func() interface{} {
		return new(bytes.Buffer)
	},
}

func recordDroppedFrame() {
	atomic.AddUint64(&droppedFrames, 1)
}

func recordSentFrame() {
	atomic.AddUint64(&totalFrames, 1)
}

func StartPipelineBroadcaster() {
	for resChan := range pipeline {
		res := <-resChan
		if res.data != nil {
			broadcastFrame(res.data)
			if firstFrameForwardedGo {
				state.wsClientsMutex.RLock()
				wsCount := len(state.wsClients)
				state.wsClientsMutex.RUnlock()
				log.Printf("First Frame Forwarded To Client")
				log.Printf("Client Count: %d", wsCount)
				firstFrameForwardedGo = false
			}
		}
	}
}

func StartBackpressureMonitor() {
	ticker := time.NewTicker(2 * time.Second)
	go func() {
		for range ticker.C {
			configMutex.RLock()
			active := currentConfig.Backpressure
			configMutex.RUnlock()

			if !active {
				continue
			}

			dropped := atomic.SwapUint64(&droppedFrames, 0)
			total := atomic.SwapUint64(&totalFrames, 0)
			all := dropped + total

			if all > 10 {
				dropRate := float64(dropped) / float64(all)
				log.Printf("[Backpressure Monitor] Total: %d, Dropped: %d, Rate: %.1f%%\n", all, dropped, dropRate*100)

				var targetFps uint32 = 60
				if dropRate > 0.4 {
					targetFps = 20
				} else if dropRate > 0.2 {
					targetFps = 30
				} else if dropRate > 0.05 {
					targetFps = 45
				}

				sendPacingToRust(targetFps)
			}
		}
	}()
}

func sendPacingToRust(fps uint32) {
	configMutex.RLock()
	config := currentConfig
	configMutex.RUnlock()

	cmd := RustCommand{
		Type:         "streamconfig",
		Backpressure: config.Backpressure,
		Codec:        config.Codec,
		TargetFps:    fps,
	}

	payload, err := json.Marshal(cmd)
	if err != nil {
		log.Printf("Error marshaling rust backpressure pacing command: %v\n", err)
		return
	}

	_ = forwardCommandToUds(payload)
}

func notifyRustDaemonOfConfig() {
	configMutex.RLock()
	config := currentConfig
	configMutex.RUnlock()

	cmd := RustCommand{
		Type:         "streamconfig",
		Backpressure: config.Backpressure,
		Codec:        config.Codec,
		TargetFps:    60,
	}

	payload, err := json.Marshal(cmd)
	if err != nil {
		log.Printf("Error marshaling rust config command: %v\n", err)
		return
	}

	_ = forwardCommandToUds(payload)
}

func countNalTypes(data []byte) (sps, pps, idr int) {
	for i := 0; i < len(data)-3; {
		if data[i] == 0 && data[i+1] == 0 {
			startLen := 0
			typeIdx := 0
			if data[i+2] == 1 {
				startLen = 3
				typeIdx = i + 3
			} else if i+3 < len(data) && data[i+2] == 0 && data[i+3] == 1 {
				startLen = 4
				typeIdx = i + 4
			} else {
				i++
				continue
			}

			if typeIdx < len(data) {
				nalType := data[typeIdx] & 0x1F
				if nalType == 7 {
					sps++
				} else if nalType == 8 {
					pps++
				} else if nalType == 5 {
					idr++
				}
			}
			i += startLen
		} else {
			i++
		}
	}
	return
}

func extractConfigNals(data []byte) []byte {
	var config []byte
	for i := 0; i < len(data)-3; {
		if data[i] == 0 && data[i+1] == 0 {
			startLen := 0
			typeIdx := 0
			if data[i+2] == 1 {
				startLen = 3
				typeIdx = i + 3
			} else if i+3 < len(data) && data[i+2] == 0 && data[i+3] == 1 {
				startLen = 4
				typeIdx = i + 4
			} else {
				i++
				continue
			}

			if typeIdx < len(data) {
				nalType := data[typeIdx] & 0x1F
				if nalType == 7 || nalType == 8 {
					startIdx := i
					endIdx := len(data)
					for j := typeIdx; j < len(data)-2; j++ {
						if data[j] == 0 && data[j+1] == 0 && (data[j+2] == 1 || (j+3 < len(data) && data[j+2] == 0 && data[j+3] == 1)) {
							endIdx = j
							break
						}
					}
					config = append(config, data[startIdx:endIdx]...)
					i = endIdx
					continue
				}
			}
			i += startLen
		} else {
			i++
		}
	}
	return config
}

func parseFrameStream(r io.Reader) error {
	header := make([]byte, 20)

	for {
		_, err := io.ReadFull(r, header)
		if err != nil {
			return err
		}

		width := binary.BigEndian.Uint32(header[0:4])
		height := binary.BigEndian.Uint32(header[4:8])
		_ = binary.BigEndian.Uint64(header[8:16])
		payloadLen := binary.BigEndian.Uint32(header[16:20])

		if firstFrameGo {
			log.Printf("DEBUG: UDS Header bytes: %x", header)
			log.Printf("DEBUG: Parsed width=%d, height=%d, payloadLen=%d", width, height, payloadLen)
		}

		if payloadLen > 32*1024*1024 {
			return fmt.Errorf("oversized frame payload rejected: %d bytes", payloadLen)
		}

		if width == 0 || height == 0 || width > 7680 || height > 4320 {
			log.Printf("Warning: skipping frame with invalid dimensions: %dx%d\n", width, height)
			if payloadLen > 0 {
				_, _ = io.CopyN(io.Discard, r, int64(payloadLen))
			}
			continue
		}

		state.wsClientsMutex.RLock()
		wsCount := len(state.wsClients)
		state.wsClientsMutex.RUnlock()

		state.rtcChannelsMutex.RLock()
		rtcCount := len(state.rtcChannels)
		state.rtcChannelsMutex.RUnlock()

		if wsCount == 0 && rtcCount == 0 {
			_, err = io.CopyN(io.Discard, r, int64(payloadLen))
			if err != nil {
				return err
			}
			continue
		}

		payload := make([]byte, payloadLen)
		_, err = io.ReadFull(r, payload)
		if err != nil {
			return err
		}

		configMutex.RLock()
		isH264 := currentConfig.Codec == "h264"
		configMutex.RUnlock()

		if firstFrameGo {
			log.Printf("First Frame Received From UDS")
			log.Printf("Payload Length: %d bytes", payloadLen)
			firstFrameGo = false
		}

		if isH264 {
			sps, pps, idr := countNalTypes(payload)
			if sps > 0 || pps > 0 || idr > 0 {
				log.Printf("Go NAL Detected: SPS=%d, PPS=%d, IDR=%d", sps, pps, idr)
			}

			if sps > 0 && pps > 0 {
				state.h264ConfigMutex.Lock()
				state.cachedSpsPps = extractConfigNals(payload)
				state.h264ConfigMutex.Unlock()
			}

			if idr > 0 && (sps == 0 || pps == 0) {
				state.h264ConfigMutex.RLock()
				cached := state.cachedSpsPps
				state.h264ConfigMutex.RUnlock()

				if len(cached) > 0 {
					log.Printf("Prepending cached SPS/PPS to naked IDR frame")
					newPayload := make([]byte, len(cached)+len(payload))
					copy(newPayload, cached)
					copy(newPayload[len(cached):], payload)
					payload = newPayload
				}
			}
		}

		resChan := make(chan encodeResult, 1)
		pipeline <- resChan

		if isH264 {
			resChan <- encodeResult{data: payload}
		} else {
			go func(p []byte, w, h uint32, ch chan encodeResult) {
				for i := 0; i < len(p); i += 4 {
					p[i], p[i+2] = p[i+2], p[i]
				}

				img := &image.RGBA{
					Pix:    p,
					Stride: int(w) * 4,
					Rect:   image.Rect(0, 0, int(w), int(h)),
				}

				bufVal := bufferPool.Get()
				jpegBuf := bufVal.(*bytes.Buffer)
				jpegBuf.Reset()
				defer bufferPool.Put(jpegBuf)

				err := jpeg.Encode(jpegBuf, img, &jpeg.EncoderOptions{Quality: JpegQuality})
				if err != nil {
					log.Printf("JPEG compression failure: %v\n", err)
					ch <- encodeResult{data: nil}
					return
				}

				compressedData := make([]byte, jpegBuf.Len())
				copy(compressedData, jpegBuf.Bytes())
				ch <- encodeResult{data: compressedData}
			}(payload, width, height, resChan)
		}
	}
}

func HandleStreamConfigAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	var newConfig StreamConfig
	if err := json.NewDecoder(r.Body).Decode(&newConfig); err != nil {
		log.Printf("Error decoding stream config: %v\n", err)
		http.Error(w, "Bad Request", http.StatusBadRequest)
		return
	}

	configMutex.Lock()
	currentConfig = newConfig
	configMutex.Unlock()

	log.Printf("Stream configuration updated: %+v\n", newConfig)

	notifyRustDaemonOfConfig()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"status":"success"}`))
}

func broadcastFrame(data []byte) {
	configMutex.RLock()
	dropFrames := currentConfig.FrameDropping
	configMutex.RUnlock()

	state.wsClientsMutex.RLock()
	for client := range state.wsClients {
		if dropFrames {
			go func(c *SafeConn) {
				sent, err := c.TryWriteMessage(websocket.BinaryMessage, data)
				if err != nil {
					removeWSClient(c)
				} else if !sent {
					recordDroppedFrame()
				} else {
					recordSentFrame()
				}
			}(client)
		} else {
			go func(c *SafeConn) {
				err := c.WriteMessage(websocket.BinaryMessage, data)
				if err != nil {
					removeWSClient(c)
				} else {
					recordSentFrame()
				}
			}(client)
		}
	}
	state.wsClientsMutex.RUnlock()

	state.rtcChannelsMutex.RLock()
	for channel := range state.rtcChannels {
		if dropFrames && channel.BufferedAmount() > 256*1024 {
			recordDroppedFrame()
			continue
		}
		go func(dc *webrtc.DataChannel) {
			err := dc.Send(data)
			if err != nil {
				state.rtcChannelsMutex.Lock()
				_, exists := state.rtcChannels[dc]
				if exists {
					delete(state.rtcChannels, dc)
					state.rtcChannelsMutex.Unlock()
					log.Printf("WebRTC Data Channel write failed. Removing channel.\n")
					dc.Close()
				} else {
					state.rtcChannelsMutex.Unlock()
				}
			} else {
				recordSentFrame()
			}
		}(channel)
	}
	state.rtcChannelsMutex.RUnlock()
}

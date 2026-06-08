package internal

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/pion/webrtc/v3"
)

type WebRtcEnvelope struct {
	SDP string `json:"sdp"`
}

func HandleWebRTCOffer(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	var envelope WebRtcEnvelope
	err := json.NewDecoder(r.Body).Decode(&envelope)
	if err != nil {
		http.Error(w, "Bad Request", http.StatusBadRequest)
		return
	}

	config := webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{"stun:stun.l.google.com:19302"},
			},
		},
	}

	peerConnection, err := webrtc.NewPeerConnection(config)
	if err != nil {
		log.Printf("Failed to instantiate WebRTC connection: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	peerConnection.OnDataChannel(func(dc *webrtc.DataChannel) {
		if dc.Label() == "commands" {
			log.Printf("WebRTC Command Data Channel opened: %s\n", dc.Label())
			dc.OnMessage(func(msg webrtc.DataChannelMessage) {
				handleCommand(msg.Data)
			})
		} else if dc.Label() == "display" {
			log.Printf("WebRTC Display Data Channel opened: %s\n", dc.Label())
			state.rtcChannelsMutex.Lock()
			state.rtcChannels[dc] = true
			state.rtcChannelsMutex.Unlock()

			dc.OnClose(func() {
				state.rtcChannelsMutex.Lock()
				delete(state.rtcChannels, dc)
				state.rtcChannelsMutex.Unlock()
				log.Println("WebRTC Display Data Channel closed.")
			})
		}
	})

	offer := webrtc.SessionDescription{
		Type: webrtc.SDPTypeOffer,
		SDP:  envelope.SDP,
	}

	err = peerConnection.SetRemoteDescription(offer)
	if err != nil {
		log.Printf("Failed to map remote WebRTC description: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	answer, err := peerConnection.CreateAnswer(nil)
	if err != nil {
		log.Printf("Failed to generate WebRTC local answer SDP: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	gatherComplete := webrtc.GatheringCompletePromise(peerConnection)
	err = peerConnection.SetLocalDescription(answer)
	if err != nil {
		log.Printf("Failed to establish local WebRTC description: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	<-gatherComplete

	responseSDP := peerConnection.LocalDescription().SDP
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(WebRtcEnvelope{SDP: responseSDP})
	log.Printf("Completed secure WebRTC SDP handshake with client!\n")
}

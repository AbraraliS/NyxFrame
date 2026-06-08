package internal

import (
	"log"
	"net"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/pion/webrtc/v3"
)

const (
	JpegQuality    = 70
	UdsRetryPeriod = 1 * time.Second
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 4096,
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

type StreamConfig struct {
	FrameDropping bool   `json:"frame_dropping"`
	Transport     string `json:"transport"`
	Backpressure  bool   `json:"backpressure"`
	Codec         string `json:"codec"`
}

type RustCommand struct {
	Type         string `json:"type"`
	Backpressure bool   `json:"backpressure"`
	Codec        string `json:"codec"`
	TargetFps    uint32 `json:"target_fps"`
}

var currentConfig = StreamConfig{
	FrameDropping: false,
	Transport:     "websocket",
	Backpressure:  false,
	Codec:         "h264",
}
var configMutex sync.RWMutex

type SafeConn struct {
	conn       *websocket.Conn
	writeMutex sync.Mutex
}

func NewSafeConn(c *websocket.Conn) *SafeConn {
	return &SafeConn{conn: c}
}

func (s *SafeConn) WriteMessage(messageType int, data []byte) error {
	s.writeMutex.Lock()
	defer s.writeMutex.Unlock()
	return s.conn.WriteMessage(messageType, data)
}

func (s *SafeConn) TryWriteMessage(messageType int, data []byte) (bool, error) {
	if !s.writeMutex.TryLock() {
		return false, nil
	}
	defer s.writeMutex.Unlock()
	err := s.conn.WriteMessage(messageType, data)
	return true, err
}

func (s *SafeConn) Close() error {
	s.writeMutex.Lock()
	defer s.writeMutex.Unlock()
	return s.conn.Close()
}

func removeWSClient(c *SafeConn) {
	state.wsClientsMutex.Lock()
	_, exists := state.wsClients[c]
	if exists {
		delete(state.wsClients, c)
		state.wsClientsMutex.Unlock()
		log.Printf("WS write failed. Removing client.\n")
		c.Close()
	} else {
		state.wsClientsMutex.Unlock()
	}
}

type ServerState struct {
	udsConn      net.Conn
	udsConnMutex sync.Mutex

	wsClients      map[*SafeConn]bool
	wsClientsMutex sync.RWMutex

	rtcChannels      map[*webrtc.DataChannel]bool
	rtcChannelsMutex sync.RWMutex

	cachedSpsPps    []byte
	h264ConfigMutex sync.RWMutex
}

var state = &ServerState{
	wsClients:   make(map[*SafeConn]bool),
	rtcChannels: make(map[*webrtc.DataChannel]bool),
}

package internal

import (
	"log"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
)

func HandleCommandWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket Command Upgrade Error: %v\n", err)
		return
	}

	log.Printf("Android client connected to input control endpoint: %s\n", conn.RemoteAddr())

	defer conn.Close()

	for {
		_, message, err := conn.ReadMessage()
		if err != nil {
			log.Printf("Command stream connection terminated: %s\n", conn.RemoteAddr())
			break
		}

		handleCommand(message)
	}
}

func HandleStreamWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WebSocket Upgrade Error: %v\n", err)
		return
	}

	log.Printf("Android client connected to video stream endpoint: %s\n", conn.RemoteAddr())

	safeConn := NewSafeConn(conn)

	state.wsClientsMutex.Lock()
	state.wsClients[safeConn] = true
	state.wsClientsMutex.Unlock()

	notifyRustDaemonOfConfig()

	defer func() {
		state.wsClientsMutex.Lock()
		delete(state.wsClients, safeConn)
		state.wsClientsMutex.Unlock()
		safeConn.Close()
		log.Printf("Video stream client disconnected: %s\n", conn.RemoteAddr())
	}()

	const streamDeadline = 90 * time.Second
	conn.SetReadDeadline(time.Now().Add(streamDeadline))
	conn.SetPingHandler(func(appData string) error {
		conn.SetReadDeadline(time.Now().Add(streamDeadline))
		return conn.WriteControl(
			websocket.PongMessage,
			[]byte(appData),
			time.Now().Add(10*time.Second),
		)
	})
	for {
		_, _, err := conn.ReadMessage()
		if err != nil {
			break
		}
	}
}

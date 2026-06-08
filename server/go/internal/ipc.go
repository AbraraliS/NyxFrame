package internal

import (
	"encoding/binary"
	"fmt"
	"log"
	"net"
	"time"
)

var udsConnectedTime time.Time

func MonitorAndProcessUDS() {
	backoff := 250 * time.Millisecond
	maxBackoff := 8 * time.Second
	for {
		log.Printf("Connecting to local systems engine over UDS: %s\n", SocketPath)
		conn, err := net.Dial("unix", SocketPath)
		if err != nil {
			log.Printf("Systems engine unreachable. Retrying in %v...\n", backoff)
			time.Sleep(backoff)
			backoff *= 2
			if backoff > maxBackoff {
				backoff = maxBackoff
			}
			continue
		}
		backoff = 250 * time.Millisecond

		log.Println("Successfully attached to systems engine Unix Domain Socket!")
		state.udsConnMutex.Lock()
		state.udsConn = conn
		state.udsConnMutex.Unlock()
		udsConnectedTime = time.Now()

		err = parseFrameStream(conn)
		if err != nil {
			log.Printf("UDS connection dropped: %v\n", err)
		}

		state.udsConnMutex.Lock()
		if state.udsConn != nil {
			state.udsConn.Close()
			state.udsConn = nil
		}
		state.udsConnMutex.Unlock()

		time.Sleep(1 * time.Second)
	}
}

func forwardCommandToUds(payload []byte) error {
	state.udsConnMutex.Lock()
	defer state.udsConnMutex.Unlock()

	if state.udsConn == nil {
		return fmt.Errorf("systems engine UDS connection is currently down")
	}

	if err := state.udsConn.SetWriteDeadline(time.Now().Add(2 * time.Second)); err != nil {
		return fmt.Errorf("failed to set write deadline: %w", err)
	}
	defer state.udsConn.SetWriteDeadline(time.Time{})

	lenBuf := make([]byte, 4)
	binary.BigEndian.PutUint32(lenBuf, uint32(len(payload)))

	_, err := state.udsConn.Write(lenBuf)
	if err != nil {
		return err
	}

	_, err = state.udsConn.Write(payload)
	if err != nil {
		return err
	}

	return nil
}

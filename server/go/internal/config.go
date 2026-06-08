package internal

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"strings"
	"sync"
	"time"
)

var (
	Port             = "9090"
	SocketPath       = "/tmp/nyxframe3.sock"
	WindowManagerCmd = "i3-msg"
)

var logFile *os.File
var logFileMutex sync.Mutex

type ServerConfig struct {
	NetworkPort      string `json:"network_port"`
	UdsSocketPath    string `json:"uds_socket_path"`
	WindowManagerCmd string `json:"window_manager_cmd"`
}

func LoadServerConfig() {
	paths := []string{"../config.json", "config.json", "server/config.json", "../server/config.json"}
	var fileBytes []byte
	var err error
	for _, p := range paths {
		fileBytes, err = os.ReadFile(p)
		if err == nil {
			break
		}
	}
	if err != nil {
		log.Printf("[INFO] config.json not found, using default configurations.\n")
		return
	}

	var conf ServerConfig
	if err := json.Unmarshal(fileBytes, &conf); err != nil {
		log.Printf("[WARNING] Failed to parse config.json: %v. Using defaults.\n", err)
		return
	}

	if conf.NetworkPort != "" {
		Port = conf.NetworkPort
	}
	if conf.UdsSocketPath != "" {
		SocketPath = conf.UdsSocketPath
	}
	if conf.WindowManagerCmd != "" {
		WindowManagerCmd = conf.WindowManagerCmd
	}
	log.Printf("[INFO] Loaded configurations: Port=%s, SocketPath=%s, WM=%s\n", Port, SocketPath, WindowManagerCmd)
}

func InitLogFile() {
	path := os.Getenv("LOG_FILE_PATH")
	if path == "" {
		return
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err == nil {
		logFile = f
		log.SetOutput(io.MultiWriter(os.Stderr, f))
	}
}

func LogToFileOnly(format string, v ...interface{}) {
	msg := fmt.Sprintf(format, v...)
	timestamp := time.Now().Format("2006/01/02 15:04:05 ")
	fullMsg := timestamp + msg
	if !strings.HasSuffix(fullMsg, "\n") {
		fullMsg += "\n"
	}

	logFileMutex.Lock()
	if logFile != nil {
		logFile.WriteString(fullMsg)
	}
	logFileMutex.Unlock()
}

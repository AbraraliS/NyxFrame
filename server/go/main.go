package main

import (
	"log"
	"net/http"
	"time"

	"nyxframe-server-go/internal"
)

func init() {
	go internal.StartXdotoolWorker()
	go internal.StartPipelineBroadcaster()
	go internal.StartBackpressureMonitor()
}

func main() {
	internal.LoadServerConfig()
	internal.InitLogFile()
	log.Println("======================================================")
	log.Println("               NYXFRAME SIGNALING DAEMON              ")
	log.Println("======================================================")

	ip := internal.GetTailscaleOrLocalIP()
	log.Printf("Binding workstation services strictly to network endpoint: %s\n", ip)

	go internal.MonitorAndProcessUDS()

	http.HandleFunc("/offer", internal.HandleWebRTCOffer)
	http.HandleFunc("/ws", internal.HandleCommandWebSocket)
	http.HandleFunc("/stream", internal.HandleStreamWebSocket)
	http.HandleFunc("/api/stream/config", internal.HandleStreamConfigAPI)
	http.HandleFunc("/api/macros/export", internal.HandleMacrosExportAPI)
	http.HandleFunc("/api/macros/import", internal.HandleMacrosImportAPI)
	http.HandleFunc("/api/fs/list", internal.HandleFsListAPI)
	http.HandleFunc("/api/fs/upload", internal.HandleFsUploadAPI)
	http.HandleFunc("/api/fs/download", internal.HandleFsDownloadAPI)
	http.HandleFunc("/api/fs/mkdir", internal.HandleFsMkdirAPI)

	addr := "0.0.0.0:" + internal.Port
	internal.FreePort(internal.Port)

	srv := &http.Server{
		Addr:         addr,
		ReadTimeout:  30 * time.Second,
		WriteTimeout: 120 * time.Second,
		IdleTimeout:  60 * time.Second,
	}
	log.Printf("Listening for client handshakes on port %s (All interfaces, including http://%s:%s)\n", internal.Port, ip, internal.Port)
	if err := srv.ListenAndServe(); err != nil {
		log.Fatalf("Fatal: Web Server crashed: %v\n", err)
	}
}

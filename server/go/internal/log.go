package internal

import (
	"fmt"
	"log"
	"sync"
)

type dedupLogger struct {
	mu      sync.Mutex
	lastMsg string
	count   int
}

func (d *dedupLogger) Printf(format string, args ...interface{}) {
	msg := fmt.Sprintf(format, args...)
	d.mu.Lock()
	defer d.mu.Unlock()
	if msg == d.lastMsg {
		d.count++
		return
	}
	if d.count > 0 {
		log.Printf("%s  ×%d\n", d.lastMsg, d.count+1)
		d.count = 0
	} else if d.lastMsg != "" {
		log.Print(d.lastMsg + "\n")
	}
	d.lastMsg = msg
}

func (d *dedupLogger) Flush() {
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.lastMsg == "" {
		return
	}
	if d.count > 0 {
		log.Printf("%s  ×%d\n", d.lastMsg, d.count+1)
	} else {
		log.Print(d.lastMsg + "\n")
	}
	d.lastMsg = ""
	d.count = 0
}

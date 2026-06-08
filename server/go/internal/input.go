package internal

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

type Command struct {
	Type    string `json:"type"`
	KeyCode uint16 `json:"key_code,omitempty"`
	Pressed bool   `json:"pressed,omitempty"`
	Dx      int32  `json:"dx,omitempty"`
	Dy      int32  `json:"dy,omitempty"`
	X       int32  `json:"x,omitempty"`
	Y       int32  `json:"y,omitempty"`
	MaxX    int32  `json:"max_x,omitempty"`
	MaxY    int32  `json:"max_y,omitempty"`
	Button  uint16 `json:"button,omitempty"`
	Steps   int32  `json:"steps,omitempty"`
	Text    string `json:"text,omitempty"`
}

var xdotoolQueue = make(chan []string, 1000)

var cmdLog = &dedupLogger{}

func StartXdotoolWorker() {
	for args := range xdotoolQueue {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		execCmd := exec.CommandContext(ctx, "xdotool", args...)
		execCmd.Env = append(os.Environ(), "DISPLAY=:0")
		if err := execCmd.Run(); err != nil {
			if ctx.Err() == context.DeadlineExceeded {
				log.Printf("Warning: xdotool %v timed out after 5s\n", args)
			} else {
				log.Printf("Warning: xdotool %v failed: %v\n", args, err)
			}
		}
		cancel()
	}
}

func handleCommand(message []byte) {
	var cmd Command
	if err := json.Unmarshal(message, &cmd); err != nil {
		log.Printf("Malformed control package received: %v\n", err)
		return
	}

	runXdotool := func(args ...string) {
		select {
		case xdotoolQueue <- args:
		default:
			log.Printf("xdotool queue full — dropping command: %v\n", args)
		}
	}

	switch cmd.Type {
	case "keyframe":
		cmdLog.Flush()
		log.Println("Force keyframe request received from client.")
		notifyRustDaemonOfConfig()

	case "workspace":
		if cmd.Text != "" {
			ws := cmd.Text
			isNumeric := true
			for _, r := range ws {
				if r < '0' || r > '9' {
					isNumeric = false
					break
				}
			}

			var i3Args []string
			if isNumeric {
				cmdLog.Printf("Workspace → %s", ws)
				i3Args = []string{"workspace", "number", ws}
			} else {
				cmdLog.Printf("Workspace → %s", ws)
				i3Args = []string{"workspace", ws}
			}
			cmdLog.Flush()

			go func(args []string) {
				execCmd := exec.Command(WindowManagerCmd, args...)
				execCmd.Env = os.Environ()
				if sockPath := getI3SocketPath(); sockPath != "" {
					execCmd.Env = append(execCmd.Env, "I3SOCK="+sockPath)
				}
				execCmd.Env = append(execCmd.Env, "DISPLAY=:0")
				if err := execCmd.Run(); err != nil {
					log.Printf("Warning: %s workspace switch failed: %v\n", WindowManagerCmd, err)
				}
			}(i3Args)
		}

	case "fullscreen":
		cmdLog.Flush()
		log.Printf("Fullscreen toggle\n")
		go func() {
			execCmd := exec.Command(WindowManagerCmd, "fullscreen", "toggle")
			execCmd.Env = os.Environ()
			if sockPath := getI3SocketPath(); sockPath != "" {
				execCmd.Env = append(execCmd.Env, "I3SOCK="+sockPath)
			}
			execCmd.Env = append(execCmd.Env, "DISPLAY=:0")
			if err := execCmd.Run(); err != nil {
				log.Printf("Warning: %s fullscreen toggle failed: %v\n", WindowManagerCmd, err)
			}
		}()

	case "text":
		if cmd.Text != "" {
			cmdLog.Flush()
			log.Printf("Type: %q\n", cmd.Text)
			runXdotool("type", "--delay", "10", cmd.Text)
		}

	case "clipboard":
		if cmd.Text != "" {
			cmdLog.Flush()
			log.Printf("Clipboard sync: %d bytes\n", len(cmd.Text))
			go func(t string) {
				execCmd1 := exec.Command("xclip", "-selection", "clipboard")
				execCmd1.Env = append(os.Environ(), "DISPLAY=:0")
				execCmd1.Stdin = strings.NewReader(t)
				if err := execCmd1.Run(); err != nil {
					log.Printf("Warning: xclip clipboard failed: %v\n", err)
				}
				execCmd2 := exec.Command("xclip", "-selection", "primary")
				execCmd2.Env = append(os.Environ(), "DISPLAY=:0")
				execCmd2.Stdin = strings.NewReader(t)
				if err := execCmd2.Run(); err != nil {
					log.Printf("Warning: xclip primary failed: %v\n", err)
				}
				time.Sleep(50 * time.Millisecond)
				runXdotool("key", "shift+Insert")
			}(cmd.Text)
		}

	case "key":
		cmdLog.Printf("Key %d %s", cmd.KeyCode+8, map[bool]string{true: "↓", false: "↑"}[cmd.Pressed])
		x11Keycode := fmt.Sprintf("%d", cmd.KeyCode+8)
		if cmd.Pressed {
			runXdotool("keydown", x11Keycode)
		} else {
			runXdotool("keyup", x11Keycode)
		}

	case "mouseabsolute":
		runXdotool("mousemove", fmt.Sprintf("%d", cmd.X), fmt.Sprintf("%d", cmd.Y))

	case "mouserelative":
		runXdotool("mousemove_relative", "--", fmt.Sprintf("%d", cmd.Dx), fmt.Sprintf("%d", cmd.Dy))

	case "mouseclick":
		buttonName := map[uint16]string{272: "LMB", 273: "RMB", 274: "MMB"}
		btn := buttonName[cmd.Button]
		if btn == "" {
			btn = fmt.Sprintf("Btn%d", cmd.Button)
		}
		cmdLog.Printf("Click %s %s", btn, map[bool]string{true: "↓", false: "↑"}[cmd.Pressed])
		var x11Button string
		switch cmd.Button {
		case 272:
			x11Button = "1"
		case 273:
			x11Button = "3"
		case 274:
			x11Button = "2"
		default:
			x11Button = "1"
		}
		if cmd.Pressed {
			runXdotool("mousedown", x11Button)
		} else {
			runXdotool("mouseup", x11Button)
		}

	case "mousescroll":
		cmdLog.Printf("Scroll %+d", cmd.Steps)
		if cmd.Steps > 0 {
			runXdotool("click", "--repeat", fmt.Sprintf("%d", cmd.Steps), "4")
		} else if cmd.Steps < 0 {
			runXdotool("click", "--repeat", fmt.Sprintf("%d", -cmd.Steps), "5")
		}
	}
}

func getI3SocketPath() string {
	matches, err := filepath.Glob("/run/user/1000/i3/ipc-socket.*")
	if err == nil && len(matches) > 0 {
		return matches[0]
	}
	matches, err = filepath.Glob("/tmp/i3-*.*/ipc-socket.*")
	if err == nil && len(matches) > 0 {
		return matches[0]
	}
	return ""
}

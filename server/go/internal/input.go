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

// runWMCmd dispatches a compositor/WM command with correct syntax per detected WM.
func runWMCmd(action string, args ...string) {
	go func() {
		var execCmd *exec.Cmd
		wm := WindowManagerCmd

		switch wm {
		case "niri":
			// niri msg action <action> [args...]
			// e.g.: niri msg action focus-workspace 1
			//       niri msg action toggle-window-fullscreen
			niriArgs := append([]string{"msg", "action", action}, args...)
			execCmd = exec.Command("niri", niriArgs...)

		case "swaymsg":
			// swaymsg [action] [args]
			full := append([]string{action}, args...)
			execCmd = exec.Command("swaymsg", full...)

		case "hyprctl":
			// hyprctl dispatch <action> [args]
			full := append([]string{"dispatch", action}, args...)
			execCmd = exec.Command("hyprctl", full...)

		default:
			// i3-msg: i3-msg [action] [args]
			full := append([]string{action}, args...)
			execCmd = exec.Command(wm, full...)
			if sockPath := getI3SocketPath(); sockPath != "" {
				execCmd.Env = append(os.Environ(), "I3SOCK="+sockPath, "DISPLAY=:0")
			}
		}

		if execCmd.Env == nil {
			execCmd.Env = append(os.Environ(), "DISPLAY=:0")
		}

		if err := execCmd.Run(); err != nil {
			log.Printf("Warning: WM command [%s %s %v] failed: %v\n", wm, action, args, err)
		}
	}()
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
			cmdLog.Printf("Workspace → %s", ws)
			cmdLog.Flush()

			switch WindowManagerCmd {
			case "niri":
				// niri msg action focus-workspace <n>  (only supports numeric)
				runWMCmd("focus-workspace", ws)
			case "swaymsg":
				runWMCmd("workspace", "number", ws)
			case "hyprctl":
				runWMCmd("workspace", ws)
			default:
				// i3-msg
				isNumeric := true
				for _, r := range ws {
					if r < '0' || r > '9' {
						isNumeric = false
						break
					}
				}
				if isNumeric {
					runWMCmd("workspace", "number", ws)
				} else {
					runWMCmd("workspace", ws)
				}
			}
		}

	case "fullscreen":
		cmdLog.Flush()
		log.Printf("Fullscreen toggle\n")
		switch WindowManagerCmd {
		case "niri":
			runWMCmd("toggle-window-fullscreen")
		case "hyprctl":
			runWMCmd("fullscreen", "0")
		default:
			// i3-msg, swaymsg
			runWMCmd("fullscreen", "toggle")
		}

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
		_ = forwardCommandToUds(message)

	case "mouseabsolute":
		_ = forwardCommandToUds(message)

	case "mouserelative":
		_ = forwardCommandToUds(message)

	case "mouseclick":
		buttonName := map[uint16]string{272: "LMB", 273: "RMB", 274: "MMB"}
		btn := buttonName[cmd.Button]
		if btn == "" {
			btn = fmt.Sprintf("Btn%d", cmd.Button)
		}
		cmdLog.Printf("Click %s %s", btn, map[bool]string{true: "↓", false: "↑"}[cmd.Pressed])
		_ = forwardCommandToUds(message)

	case "mousescroll":
		cmdLog.Printf("Scroll %+d", cmd.Steps)
		_ = forwardCommandToUds(message)
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

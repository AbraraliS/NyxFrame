package internal

import (
	"log"
	"net"
	"os/exec"
	"strings"
	"time"
)

func GetTailscaleOrLocalIP() string {
	interfaces, err := net.Interfaces()
	if err != nil {
		log.Printf("Warning: Failed to fetch network interfaces: %v\n", err)
		return "0.0.0.0"
	}

	for _, iface := range interfaces {
		if strings.HasPrefix(iface.Name, "tailscale") || strings.Contains(iface.Name, "tun") {
			addrs, err := iface.Addrs()
			if err != nil {
				continue
			}
			for _, addr := range addrs {
				var ip net.IP
				switch v := addr.(type) {
				case *net.IPNet:
					ip = v.IP
				case *net.IPAddr:
					ip = v.IP
				}
				if ip == nil || ip.IsLoopback() {
					continue
				}
				ip4 := ip.To4()
				if ip4 != nil {
					return ip4.String()
				}
			}
		}
	}

	addrs, err := net.InterfaceAddrs()
	if err == nil {
		for _, addr := range addrs {
			if ipnet, ok := addr.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
				ip4 := ipnet.IP.To4()
				if ip4 != nil {
					if ip4[0] == 100 && (ip4[1]&0xC0) == 64 {
						return ip4.String()
					}
				}
			}
		}
	}

	return "0.0.0.0"
}

func FreePort(port string) {
	portSpec := port + "/tcp"
	cmd := exec.Command("fuser", "-k", portSpec)
	if out, err := cmd.CombinedOutput(); err == nil {
		log.Printf("[Startup] Evicted stale process from port %s. Waiting for socket release...\n", port)
		time.Sleep(300 * time.Millisecond)
	} else {
		_ = out
	}
}

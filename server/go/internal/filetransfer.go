package internal

import (
	"archive/zip"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync/atomic"
	"time"
)

type FsItem struct {
	Name  string `json:"name"`
	Path  string `json:"path"`
	IsDir bool   `json:"isDir"`
	Size  int64  `json:"size"`
}

type FsListResponse struct {
	CurrentPath string   `json:"currentPath"`
	ParentPath  string   `json:"parentPath"`
	Items       []FsItem `json:"items"`
}

type FsDownloadPayload struct {
	Paths []string `json:"paths"`
}

type trackingReader struct {
	r    io.Reader
	read *int64
}

func (tr *trackingReader) Read(p []byte) (n int, err error) {
	n, err = tr.r.Read(p)
	atomic.AddInt64(tr.read, int64(n))
	return
}

type trackingResponseWriter struct {
	http.ResponseWriter
	written *int64
}

func (trw *trackingResponseWriter) Write(p []byte) (n int, err error) {
	n, err = trw.ResponseWriter.Write(p)
	atomic.AddInt64(trw.written, int64(n))
	return
}

func HandleFsListAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	targetPath := r.URL.Query().Get("path")
	if targetPath == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			log.Printf("Error resolving home directory: %v\n", err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}
		targetPath = home
	}

	targetPath = filepath.Clean(targetPath)
	dirEntries, err := os.ReadDir(targetPath)
	if err != nil {
		log.Printf("Error reading directory %s: %v\n", targetPath, err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	items := make([]FsItem, 0, len(dirEntries))
	for _, entry := range dirEntries {
		name := entry.Name()
		if strings.HasPrefix(name, ".") {
			continue
		}
		info, err := entry.Info()
		var size int64 = 0
		if err == nil {
			size = info.Size()
		}
		absPath := filepath.Join(targetPath, name)
		items = append(items, FsItem{
			Name:  name,
			Path:  absPath,
			IsDir: entry.IsDir(),
			Size:  size,
		})
	}

	parentPath := filepath.Dir(targetPath)
	if parentPath == targetPath {
		parentPath = ""
	}

	resp := FsListResponse{
		CurrentPath: targetPath,
		ParentPath:  parentPath,
		Items:       items,
	}

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(resp); err != nil {
		log.Printf("Error encoding fs list: %v\n", err)
	}
}

func HandleFsUploadAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	dest := r.URL.Query().Get("dest")
	if dest == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			log.Printf("Error resolving home: %v\n", err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}
		dest = home
	}
	dest = filepath.Clean(dest)

	var bytesRead int64
	totalBytes := r.ContentLength
	doneChan := make(chan struct{})
	ticker := time.NewTicker(10 * time.Second)
	go func() {
		for {
			select {
			case <-ticker.C:
				read := atomic.LoadInt64(&bytesRead)
				var left int64
				if totalBytes > 0 {
					left = totalBytes - read
					if left < 0 {
						left = 0
					}
					log.Printf("[UPLOAD PROGRESS] Got %.2f MB / %.2f MB (%.1f%%), %.2f MB left\n",
						float64(read)/(1024*1024),
						float64(totalBytes)/(1024*1024),
						float64(read)*100.0/float64(totalBytes),
						float64(left)/(1024*1024))
				} else {
					log.Printf("[UPLOAD PROGRESS] Got %.2f MB (total size unknown)\n",
						float64(read)/(1024*1024))
				}
			case <-doneChan:
				ticker.Stop()
				return
			}
		}
	}()
	defer close(doneChan)

	r.Body = io.NopCloser(&trackingReader{r: r.Body, read: &bytesRead})

	mr, err := r.MultipartReader()
	if err != nil {
		log.Printf("Error reading multipart body: %v\n", err)
		http.Error(w, "Bad Request", http.StatusBadRequest)
		return
	}

	for {
		part, err := mr.NextPart()
		if err == io.EOF {
			break
		}
		if err != nil {
			log.Printf("Error reading part: %v\n", err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}

		fileName := part.FileName()
		if fileName == "" {
			continue
		}

		targetFile := filepath.Clean(filepath.Join(dest, fileName))
		if !strings.HasPrefix(targetFile, dest) {
			log.Printf("Warning: blocked directory traversal attempt: %s\n", targetFile)
			continue
		}

		parentDir := filepath.Dir(targetFile)
		if err := os.MkdirAll(parentDir, 0755); err != nil {
			log.Printf("Error creating subfolders: %v\n", err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}

		f, err := os.OpenFile(targetFile, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0644)
		if err != nil {
			log.Printf("Error creating file %s: %v\n", targetFile, err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}

		_, err = io.Copy(f, part)
		f.Close()
		if err != nil {
			log.Printf("Error writing file %s: %v\n", targetFile, err)
			http.Error(w, "Internal Server Error", http.StatusInternalServerError)
			return
		}
	}

	log.Printf("✓ File sharing upload successfully saved to target folder: %s\n", dest)
	w.WriteHeader(http.StatusOK)
}

func HandleFsDownloadAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	var payload FsDownloadPayload
	if err := json.NewDecoder(r.Body).Decode(&payload); err != nil || len(payload.Paths) == 0 {
		log.Printf("Error decoding download payload: %v\n", err)
		http.Error(w, "Bad Request", http.StatusBadRequest)
		return
	}

	var totalBytes int64
	for _, target := range payload.Paths {
		target = filepath.Clean(target)
		info, err := os.Stat(target)
		if err != nil {
			continue
		}
		if info.IsDir() {
			filepath.Walk(target, func(path string, fileInfo os.FileInfo, walkErr error) error {
				if walkErr == nil && !fileInfo.IsDir() {
					totalBytes += fileInfo.Size()
				}
				return nil
			})
		} else {
			totalBytes += info.Size()
		}
	}

	var bytesWritten int64
	doneChan := make(chan struct{})
	ticker := time.NewTicker(10 * time.Second)
	go func() {
		for {
			select {
			case <-ticker.C:
				written := atomic.LoadInt64(&bytesWritten)
				var left int64
				if totalBytes > 0 {
					left = totalBytes - written
					if left < 0 {
						left = 0
					}
					log.Printf("[DOWNLOAD PROGRESS] Sent %.2f MB / %.2f MB (%.1f%%), %.2f MB left\n",
						float64(written)/(1024*1024),
						float64(totalBytes)/(1024*1024),
						float64(written)*100.0/float64(totalBytes),
						float64(left)/(1024*1024))
				} else {
					log.Printf("[DOWNLOAD PROGRESS] Sent %.2f MB\n",
						float64(written)/(1024*1024))
				}
			case <-doneChan:
				ticker.Stop()
				return
			}
		}
	}()
	defer close(doneChan)

	trw := &trackingResponseWriter{ResponseWriter: w, written: &bytesWritten}

	if len(payload.Paths) == 1 {
		singlePath := filepath.Clean(payload.Paths[0])
		info, err := os.Stat(singlePath)
		if err == nil && !info.IsDir() {
			trw.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%q", filepath.Base(singlePath)))
			trw.Header().Set("Content-Type", "application/octet-stream")
			http.ServeFile(trw, r, singlePath)
			return
		}
	}

	zipName := "download.zip"
	if len(payload.Paths) == 1 {
		zipName = filepath.Base(payload.Paths[0]) + ".zip"
	}

	trw.Header().Set("Content-Type", "application/zip")
	trw.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=%q", zipName))

	zw := zip.NewWriter(trw)
	defer zw.Close()

	for _, target := range payload.Paths {
		target = filepath.Clean(target)
		info, err := os.Stat(target)
		if err != nil {
			log.Printf("Download target not found: %s\n", target)
			continue
		}

		baseDir := filepath.Dir(target)

		if info.IsDir() {
			err = filepath.Walk(target, func(path string, fileInfo os.FileInfo, walkErr error) error {
				if walkErr != nil {
					return walkErr
				}

				relPath, err := filepath.Rel(baseDir, path)
				if err != nil {
					return err
				}

				header, err := zip.FileInfoHeader(fileInfo)
				if err != nil {
					return err
				}

				header.Name = relPath
				if fileInfo.IsDir() {
					header.Name += "/"
				} else {
					header.Method = zip.Deflate
				}

				writer, err := zw.CreateHeader(header)
				if err != nil {
					return err
				}

				if !fileInfo.IsDir() {
					f, err := os.Open(path)
					if err != nil {
						return err
					}
					defer f.Close()
					_, err = io.Copy(writer, f)
					if err != nil {
						return err
					}
				}
				return nil
			})
			if err != nil {
				log.Printf("Error walking folder %s: %v\n", target, err)
			}
		} else {
			relPath := filepath.Base(target)
			header, err := zip.FileInfoHeader(info)
			if err != nil {
				continue
			}
			header.Name = relPath
			header.Method = zip.Deflate

			writer, err := zw.CreateHeader(header)
			if err != nil {
				continue
			}

			f, err := os.Open(target)
			if err != nil {
				continue
			}
			_, err = io.Copy(writer, f)
			f.Close()
		}
	}
	log.Printf("✓ File sharing dynamic ZIP download completed successfully!\n")
}

func HandleFsMkdirAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	path := r.URL.Query().Get("path")
	if path == "" {
		http.Error(w, "Missing path parameter", http.StatusBadRequest)
		return
	}
	path = filepath.Clean(path)

	if err := os.MkdirAll(path, 0755); err != nil {
		log.Printf("Error creating folder %s: %v\n", path, err)
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	log.Printf("✓ Securely created remote directory: %s\n", path)
	w.WriteHeader(http.StatusOK)
}

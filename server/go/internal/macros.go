package internal

import (
	"encoding/json"
	"log"
	"net/http"
	"os"
	"path/filepath"
)

type MacrosExportPayload struct {
	JSON string `json:"json"`
	TOML string `json:"toml"`
	YAML string `json:"yaml"`
}

func HandleMacrosExportAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	var payload MacrosExportPayload
	if err := json.NewDecoder(r.Body).Decode(&payload); err != nil {
		log.Printf("Error decoding macros export payload: %v\n", err)
		http.Error(w, "Bad Request", http.StatusBadRequest)
		return
	}

	exportDir := "./export"
	if err := os.MkdirAll(exportDir, 0755); err != nil {
		log.Printf("Error creating export directory: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	if err := os.WriteFile(filepath.Join(exportDir, "macros.json"), []byte(payload.JSON), 0644); err != nil {
		log.Printf("Error writing macros.json: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	if err := os.WriteFile(filepath.Join(exportDir, "macros.toml"), []byte(payload.TOML), 0644); err != nil {
		log.Printf("Error writing macros.toml: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	if err := os.WriteFile(filepath.Join(exportDir, "macros.yaml"), []byte(payload.YAML), 0644); err != nil {
		log.Printf("Error writing macros.yaml: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	log.Printf("✓ Custom macros successfully exported to server in JSON, TOML, and YAML formats!\n")
	w.WriteHeader(http.StatusOK)
}

func HandleMacrosImportAPI(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method Not Allowed", http.StatusMethodNotAllowed)
		return
	}

	jsonPath := filepath.Join("./export", "macros.json")
	if _, err := os.Stat(jsonPath); os.IsNotExist(err) {
		log.Println("Import requested but no exported macros found on server.")
		http.Error(w, "Exported macros not found on server", http.StatusNotFound)
		return
	}

	data, err := os.ReadFile(jsonPath)
	if err != nil {
		log.Printf("Error reading macros.json: %v\n", err)
		http.Error(w, "Internal Server Error", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.Write(data)
	log.Println("✓ Custom macros successfully imported from server!")
}

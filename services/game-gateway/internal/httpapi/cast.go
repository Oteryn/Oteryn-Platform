package httpapi

import (
	"io"
	"net/http"
)

// EnableCastWatch installs a separate, account-only observer issuer. Login and
// character selection are never fallbacks for this endpoint.
func (s *Server) EnableCastWatch(handler NativeLoginHandler) { s.cast = handler }
func (s *Server) watchCast(w http.ResponseWriter, r *http.Request) {
	setSensitiveResponseNoCache(w)
	if s.cast == nil {
		writeJSON(w, http.StatusServiceUnavailable, map[string]any{"error": "Cast unavailable"})
		return
	}
	if r.URL.RawQuery != "" {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "Cast unavailable"})
		return
	}
	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
	if err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]any{"error": "Cast unavailable"})
		return
	}
	s.cast.Serve(w, r, body)
}

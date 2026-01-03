//go:build cgo

package charybdis

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/lib/darwin_arm64 -lcharybdis -ldl -lm -framework Security -framework CoreFoundation -L/opt/homebrew/opt/openssl@3/lib -lssl -lcrypto
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/lib/darwin_amd64 -lcharybdis -ldl -lm -framework Security -framework CoreFoundation -L/usr/local/opt/openssl@3/lib -lssl -lcrypto
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/lib/linux_amd64 -lcharybdis -ldl -lm -lpthread -lssl -lcrypto

#include <stdlib.h>
#include "charybdis.h"

// Forward declaration - implemented in Go
// Note: Using char* instead of const char* for CGO compatibility
extern void goCharybdisProgressCallback(char* json, void* user_data);

// Wrapper that can be passed as CharybdisProgressCallback
static void progressCallbackWrapper(const char* json, void* user_data) {
    goCharybdisProgressCallback((char*)json, user_data);
}

// Get the wrapper function pointer
static CharybdisProgressCallback getProgressCallback() {
    return progressCallbackWrapper;
}
*/
import "C"

import (
	"context"
	"encoding/json"
	"log"
	"runtime/cgo"
	"strings"
	"sync"
	"unsafe"
)

// Session callback registry
var (
	callbackMu   sync.RWMutex
	callbackMap          = make(map[uintptr]ProgressCallback)
	callbackNext uintptr = 1
)

func registerCallback(cb ProgressCallback) uintptr {
	callbackMu.Lock()
	defer callbackMu.Unlock()
	id := callbackNext
	callbackNext++
	callbackMap[id] = cb
	return id
}

func unregisterCallback(id uintptr) {
	callbackMu.Lock()
	defer callbackMu.Unlock()
	delete(callbackMap, id)
}

func getCallback(id uintptr) ProgressCallback {
	callbackMu.RLock()
	defer callbackMu.RUnlock()
	return callbackMap[id]
}

//export goCharybdisProgressCallback
func goCharybdisProgressCallback(jsonC *C.char, userData unsafe.Pointer) {
	id := uintptr(userData)
	log.Printf("[Charybdis FFI] Progress callback received, id=%d", id)
	cb := getCallback(id)
	if cb == nil {
		log.Printf("[Charybdis FFI] No callback registered for id=%d", id)
		return
	}

	jsonStr := C.GoString(jsonC)
	log.Printf("[Charybdis FFI] Progress JSON: %s", jsonStr)
	var event ProgressEvent
	if err := json.Unmarshal([]byte(jsonStr), &event); err != nil {
		log.Printf("[Charybdis FFI] Failed to unmarshal progress: %v", err)
		return
	}
	log.Printf("[Charybdis FFI] Calling Go callback with phase=%s", event.Phase)
	cb(event)
}

// init initializes the session by creating the underlying Rust session
func (s *Session) init() error {
	log.Printf("[Charybdis FFI] init() called")
	configJSON, err := json.Marshal(s.config)
	if err != nil {
		log.Printf("[Charybdis FFI] Failed to marshal config: %v", err)
		return err
	}
	log.Printf("[Charybdis FFI] Config JSON: %s", string(configJSON))

	workloadJSON, err := json.Marshal(s.workload)
	if err != nil {
		log.Printf("[Charybdis FFI] Failed to marshal workload: %v", err)
		return err
	}
	log.Printf("[Charybdis FFI] Workload JSON length: %d", len(workloadJSON))

	configC := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(configC))
	workloadC := C.CString(string(workloadJSON))
	defer C.free(unsafe.Pointer(workloadC))

	log.Printf("[Charybdis FFI] Calling charybdis_session_new...")
	ptr := C.charybdis_session_new(configC, workloadC)
	if ptr == nil {
		log.Printf("[Charybdis FFI] charybdis_session_new returned NULL")
		return ErrInvalidConfig
	}
	log.Printf("[Charybdis FFI] Session created successfully, ptr=%p", ptr)

	s.ptr = uintptr(unsafe.Pointer(ptr))
	return nil
}

// run executes the benchmark with progress callback
func (s *Session) run(ctx context.Context, callback ProgressCallback) (*BenchmarkResults, error) {
	log.Printf("[Charybdis FFI] run() called, closed=%v, ptr=%d", s.closed, s.ptr)
	if s.closed {
		log.Printf("[Charybdis FFI] Session is closed")
		return nil, ErrSessionClosed
	}

	ptr := (*C.CharybdisSession)(unsafe.Pointer(s.ptr))
	log.Printf("[Charybdis FFI] Session ptr=%p", ptr)

	// Start a goroutine to handle context cancellation
	done := make(chan struct{})
	defer close(done)
	go func() {
		select {
		case <-ctx.Done():
			log.Printf("[Charybdis FFI] Context cancelled, calling cancel()")
			s.cancel()
		case <-done:
			// Benchmark finished normally
		}
	}()

	var resultC *C.char
	if callback != nil {
		cbID := registerCallback(callback)
		log.Printf("[Charybdis FFI] Registered callback with ID=%d", cbID)
		defer unregisterCallback(cbID)
		log.Printf("[Charybdis FFI] Calling charybdis_session_run with callback...")
		resultC = C.charybdis_session_run(ptr, C.getProgressCallback(), unsafe.Pointer(cbID))
	} else {
		log.Printf("[Charybdis FFI] Calling charybdis_session_run without callback...")
		resultC = C.charybdis_session_run(ptr, nil, nil)
	}
	log.Printf("[Charybdis FFI] charybdis_session_run returned, resultC=%p", resultC)

	if resultC == nil {
		log.Printf("[Charybdis FFI] Result is NULL - connection failed")
		return nil, ErrConnectionFailed
	}
	defer C.charybdis_free_string(resultC)

	resultJSON := C.GoString(resultC)
	log.Printf("[Charybdis FFI] Result JSON length: %d", len(resultJSON))
	var results BenchmarkResults
	if err := json.Unmarshal([]byte(resultJSON), &results); err != nil {
		log.Printf("[Charybdis FFI] Failed to unmarshal results: %v", err)
		return nil, err
	}
	log.Printf("[Charybdis FFI] Benchmark completed, totalOps=%d, opsPerSec=%.2f", results.TotalOps, results.OpsPerSecond)

	return &results, nil
}

// cancel requests cancellation of the benchmark
func (s *Session) cancel() {
	log.Printf("[Charybdis FFI] cancel() called, ptr=%d", s.ptr)
	if s.ptr != 0 {
		ptr := (*C.CharybdisSession)(unsafe.Pointer(s.ptr))
		log.Printf("[Charybdis FFI] Calling charybdis_session_cancel...")
		C.charybdis_session_cancel(ptr)
		log.Printf("[Charybdis FFI] charybdis_session_cancel returned")
	}
}

// free releases the session resources
func (s *Session) free() {
	if s.ptr != 0 {
		ptr := (*C.CharybdisSession)(unsafe.Pointer(s.ptr))
		C.charybdis_session_free(ptr)
		s.ptr = 0
	}
}

// version returns the charybdis library version
func version() string {
	versionC := C.charybdis_version()
	if versionC == nil {
		return "unknown"
	}
	defer C.charybdis_free_string(versionC)
	return C.GoString(versionC)
}

// validateScript validates a Rune workload script
func validateScript(script string) ([]ScriptError, error) {
	scriptC := C.CString(script)
	defer C.free(unsafe.Pointer(scriptC))

	errorsC := C.charybdis_validate_script(scriptC)
	if errorsC == nil {
		return nil, nil // valid
	}
	defer C.charybdis_free_string(errorsC)

	errorsJSON := C.GoString(errorsC)

	// Handle DIAG_JSON: prefix from structured diagnostics
	const diagPrefix = "DIAG_JSON:"
	errorsJSON = strings.TrimPrefix(errorsJSON, diagPrefix)

	var errors []ScriptError
	if err := json.Unmarshal([]byte(errorsJSON), &errors); err != nil {
		// If JSON parsing fails, return a generic error with the raw message
		return []ScriptError{{
			Line:     1,
			Column:   1,
			Message:  errorsJSON,
			Severity: "error",
		}}, ErrScriptError
	}

	return errors, ErrScriptError
}

// builtinWorkloads returns the list of built-in workload templates
func builtinWorkloads() ([]WorkloadTemplate, error) {
	templatesC := C.charybdis_builtin_workloads()
	if templatesC == nil {
		return nil, nil
	}
	defer C.charybdis_free_string(templatesC)

	templatesJSON := C.GoString(templatesC)
	var templates []WorkloadTemplate
	if err := json.Unmarshal([]byte(templatesJSON), &templates); err != nil {
		return nil, err
	}

	return templates, nil
}

// Ensure cgo.Handle is imported (for future use if needed)
var _ = cgo.Handle(0)

// Package charybdis provides Go bindings for the charybdis ScyllaDB benchmarking library.
// It wraps the charybdis-ffi Rust crate via CGO, providing structured progress callbacks,
// cancellation support, and type-safe access to benchmark results.
package charybdis

import (
	"encoding/json"
	"time"
)

// Consistency represents CQL consistency levels
// Values must match Rust's serde(rename_all = "snake_case")
type Consistency string

const (
	ConsistencyAny         Consistency = "any"
	ConsistencyOne         Consistency = "one"
	ConsistencyTwo         Consistency = "two"
	ConsistencyThree       Consistency = "three"
	ConsistencyQuorum      Consistency = "quorum"
	ConsistencyAll         Consistency = "all"
	ConsistencyLocalOne    Consistency = "local_one"
	ConsistencyLocalQuorum Consistency = "local_quorum"
	ConsistencyEachQuorum  Consistency = "each_quorum"
	ConsistencySerial      Consistency = "serial"
	ConsistencyLocalSerial Consistency = "local_serial"
)

// BenchmarkConfig configures the benchmark execution parameters
type BenchmarkConfig struct {
	// Hosts is the list of ScyllaDB hosts to connect to
	Hosts []string `json:"hosts"`

	// Port is the CQL port (default: 9042)
	Port int `json:"port,omitempty"`

	// Username for authentication (optional)
	Username string `json:"username,omitempty"`

	// Password for authentication (optional)
	Password string `json:"password,omitempty"`

	// Datacenter for DC-aware load balancing (optional)
	Datacenter string `json:"datacenter,omitempty"`

	// Threads is the number of OS threads (default: 1)
	Threads int `json:"threads,omitempty"`

	// Concurrency is the number of concurrent operations per thread
	Concurrency int64 `json:"concurrency,omitempty"`

	// RateLimit is the max operations per second, 0 = unlimited
	RateLimit float64 `json:"rate_limit,omitempty"`

	// Duration is how long to run the benchmark
	Duration time.Duration `json:"-"` // Custom marshaling below

	// Warmup is how long to warm up before measuring
	Warmup time.Duration `json:"-"` // Custom marshaling below

	// Consistency is the CQL consistency level
	Consistency Consistency `json:"consistency,omitempty"`

	// ConnectionsPerHost is the number of connections per host
	ConnectionsPerHost int `json:"connections_per_host,omitempty"`

	// SSL enables SSL/TLS connection
	SSL bool `json:"ssl,omitempty"`

	// SSLCertPath is the path to the SSL certificate
	SSLCertPath string `json:"ssl_cert_path,omitempty"`

	// SSLKeyPath is the path to the SSL key
	SSLKeyPath string `json:"ssl_key_path,omitempty"`

	// SSLCAPath is the path to the CA certificate
	SSLCAPath string `json:"ssl_ca_path,omitempty"`

	// SamplingInterval is how often to emit progress events
	SamplingInterval time.Duration `json:"-"` // Custom marshaling below
}

// benchmarkConfigJSON is the JSON representation with durations as seconds
type benchmarkConfigJSON struct {
	Hosts              []string    `json:"hosts"`
	Port               int         `json:"port,omitempty"`
	Username           string      `json:"username,omitempty"`
	Password           string      `json:"password,omitempty"`
	Datacenter         string      `json:"datacenter,omitempty"`
	Threads            int         `json:"threads,omitempty"`
	Concurrency        int64       `json:"concurrency,omitempty"`
	RateLimit          float64     `json:"rate_limit,omitempty"`
	Duration           uint64      `json:"duration,omitempty"`           // seconds
	Warmup             uint64      `json:"warmup,omitempty"`             // seconds
	SamplingInterval   uint64      `json:"sampling_interval,omitempty"` // seconds
	Consistency        Consistency `json:"consistency,omitempty"`
	ConnectionsPerHost int         `json:"connections_per_host,omitempty"`
	SSL                bool        `json:"ssl,omitempty"`
	SSLCertPath        string      `json:"ssl_cert_path,omitempty"`
	SSLKeyPath         string      `json:"ssl_key_path,omitempty"`
	SSLCAPath          string      `json:"ssl_ca_path,omitempty"`
}

// MarshalJSON implements json.Marshaler with durations as seconds
func (c BenchmarkConfig) MarshalJSON() ([]byte, error) {
	return json.Marshal(benchmarkConfigJSON{
		Hosts:              c.Hosts,
		Port:               c.Port,
		Username:           c.Username,
		Password:           c.Password,
		Datacenter:         c.Datacenter,
		Threads:            c.Threads,
		Concurrency:        c.Concurrency,
		RateLimit:          c.RateLimit,
		Duration:           uint64(c.Duration.Seconds()),
		Warmup:             uint64(c.Warmup.Seconds()),
		SamplingInterval:   uint64(c.SamplingInterval.Seconds()),
		Consistency:        c.Consistency,
		ConnectionsPerHost: c.ConnectionsPerHost,
		SSL:                c.SSL,
		SSLCertPath:        c.SSLCertPath,
		SSLKeyPath:         c.SSLKeyPath,
		SSLCAPath:          c.SSLCAPath,
	})
}

// WorkloadConfig configures the workload to run
type WorkloadConfig struct {
	// Script is the Rune script content
	Script string `json:"script"`

	// Params are parameters passed to the script
	Params map[string]string `json:"params,omitempty"`
}

// Phase represents the current benchmark phase
type Phase string

const (
	PhasePreparing Phase = "preparing"
	PhaseConnecting Phase = "connecting"
	PhaseSchema     Phase = "schema"
	PhasePrepare    Phase = "prepare"
	PhaseLoading    Phase = "loading"
	PhaseWarming    Phase = "warming"
	PhaseRunning    Phase = "running"
	PhaseCompleted  Phase = "completed"
	PhaseFailed     Phase = "failed"
	PhaseCancelled  Phase = "cancelled"
)

// RustDuration matches Rust's std::time::Duration JSON serialization
type RustDuration struct {
	Secs  uint64 `json:"secs"`
	Nanos uint32 `json:"nanos"`
}

// Milliseconds converts to milliseconds
func (d RustDuration) Milliseconds() int64 {
	return int64(d.Secs)*1000 + int64(d.Nanos)/1_000_000
}

// IntervalStats contains statistics for a sampling interval
type IntervalStats struct {
	OpsCount     int64               `json:"ops_count"`
	OpsPerSecond float64             `json:"ops_per_second"`
	ErrorCount   int64               `json:"error_count"`
	ErrorRate    float64             `json:"error_rate"`
	Latency      IntervalLatencyStats `json:"latency"`
}

// IntervalLatencyStats contains latency stats for an interval
type IntervalLatencyStats struct {
	MeanUs int64 `json:"mean_us"`
	MinUs  int64 `json:"min_us"`
	MaxUs  int64 `json:"max_us"`
	P50Us  int64 `json:"p50_us"`
	P95Us  int64 `json:"p95_us"`
	P99Us  int64 `json:"p99_us"`
	P999Us int64 `json:"p999_us"`
}

// ProgressEvent is emitted during benchmark execution (matches Rust struct)
type ProgressEvent struct {
	// SessionID is the benchmark session ID
	SessionID string `json:"session_id"`

	// Phase is the current phase
	Phase Phase `json:"phase"`

	// Elapsed is time since benchmark start
	Elapsed RustDuration `json:"elapsed"`

	// TotalDuration is the expected total duration (if known)
	TotalDuration *RustDuration `json:"total_duration"`

	// Stats contains interval statistics (if in running phase)
	Stats *IntervalStats `json:"stats"`

	// Message is an optional status message
	Message string `json:"message,omitempty"`
}

// ElapsedMs returns elapsed time in milliseconds
func (e ProgressEvent) ElapsedMs() int64 {
	return e.Elapsed.Milliseconds()
}

// OpsCompleted returns total operations (from stats)
func (e ProgressEvent) OpsCompleted() int64 {
	if e.Stats != nil {
		return e.Stats.OpsCount
	}
	return 0
}

// OpsPerSecond returns current ops/sec (from stats)
func (e ProgressEvent) OpsPerSecond() float64 {
	if e.Stats != nil {
		return e.Stats.OpsPerSecond
	}
	return 0
}

// Errors returns error count (from stats)
func (e ProgressEvent) Errors() int64 {
	if e.Stats != nil {
		return e.Stats.ErrorCount
	}
	return 0
}

// LatencyP50Us returns p50 latency (from stats)
func (e ProgressEvent) LatencyP50Us() int64 {
	if e.Stats != nil {
		return e.Stats.Latency.P50Us
	}
	return 0
}

// LatencyP95Us returns p95 latency (from stats)
func (e ProgressEvent) LatencyP95Us() int64 {
	if e.Stats != nil {
		return e.Stats.Latency.P95Us
	}
	return 0
}

// LatencyP99Us returns p99 latency (from stats)
func (e ProgressEvent) LatencyP99Us() int64 {
	if e.Stats != nil {
		return e.Stats.Latency.P99Us
	}
	return 0
}

// LatencyP999Us returns p999 latency (from stats)
func (e ProgressEvent) LatencyP999Us() int64 {
	if e.Stats != nil {
		return e.Stats.Latency.P999Us
	}
	return 0
}

// BenchmarkResults contains the final benchmark results
type BenchmarkResults struct {
	// TotalOps is the total number of operations completed
	TotalOps int64 `json:"total_ops"`

	// OpsPerSecond is the average operations per second
	OpsPerSecond float64 `json:"ops_per_second"`

	// ErrorCount is the total number of errors
	ErrorCount int64 `json:"error_count"`

	// ErrorRate is errors per operation (0.0 - 1.0)
	ErrorRate float64 `json:"error_rate"`

	// DurationMs is the actual benchmark duration in milliseconds
	DurationMs int64 `json:"duration_ms"`

	// Latency contains latency statistics
	Latency LatencyStats `json:"latency"`

	// Histogram contains the latency distribution
	Histogram []HistogramBucket `json:"histogram,omitempty"`

	// Timeseries contains ops/sec over time
	Timeseries []TimeseriesPoint `json:"timeseries,omitempty"`

	// ErrorsByType breaks down errors by type
	ErrorsByType map[string]int64 `json:"errors_by_type,omitempty"`
}

// LatencyStats contains latency statistics in microseconds
type LatencyStats struct {
	// MeanUs is the mean latency
	MeanUs int64 `json:"mean_us"`

	// MinUs is the minimum latency
	MinUs int64 `json:"min_us"`

	// MaxUs is the maximum latency
	MaxUs int64 `json:"max_us"`

	// StdDevUs is the standard deviation
	StdDevUs int64 `json:"std_dev_us"`

	// P50Us is the 50th percentile (median)
	P50Us int64 `json:"p50_us"`

	// P75Us is the 75th percentile
	P75Us int64 `json:"p75_us"`

	// P90Us is the 90th percentile
	P90Us int64 `json:"p90_us"`

	// P95Us is the 95th percentile
	P95Us int64 `json:"p95_us"`

	// P99Us is the 99th percentile
	P99Us int64 `json:"p99_us"`

	// P999Us is the 99.9th percentile
	P999Us int64 `json:"p999_us"`
}

// HistogramBucket represents a bucket in the latency histogram
type HistogramBucket struct {
	// LowerUs is the lower bound in microseconds (inclusive)
	LowerUs int64 `json:"lower_us"`

	// UpperUs is the upper bound in microseconds (exclusive)
	UpperUs int64 `json:"upper_us"`

	// Count is the number of operations in this bucket
	Count int64 `json:"count"`
}

// TimeseriesPoint is a single point in the throughput timeseries
type TimeseriesPoint struct {
	// TimestampMs is milliseconds since benchmark start
	TimestampMs int64 `json:"timestamp_ms"`

	// OpsPerSecond is the ops/sec at this point
	OpsPerSecond float64 `json:"ops_per_second"`

	// LatencyP99Us is the p99 latency at this point
	LatencyP99Us int64 `json:"latency_p99_us"`
}

// WorkloadTemplate represents a built-in workload template
type WorkloadTemplate struct {
	// Name is the template identifier
	Name string `json:"name"`

	// Description explains what the workload does
	Description string `json:"description"`

	// Script is the Rune script content
	Script string `json:"script"`

	// DefaultParams are suggested parameter values
	DefaultParams map[string]string `json:"default_params,omitempty"`
}

// ScriptError represents a validation error in a Rune script
type ScriptError struct {
	// Line is the 1-indexed line number
	Line int `json:"line"`

	// Column is the 1-indexed column number
	Column int `json:"column"`

	// Message describes the error
	Message string `json:"message"`

	// Severity is "error" or "warning"
	Severity string `json:"severity"`
}

// ProgressCallback is called with progress events during benchmark execution
type ProgressCallback func(event ProgressEvent)

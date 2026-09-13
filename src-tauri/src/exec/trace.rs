use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// A phase of one request. Which of these an executor can actually time
/// depends on the transport: the reqwest-based one sees the exchange from
/// the outside (`Wait`, `Download`), while a future hyper-based one drives
/// each step itself and can fill in `Dns`, `Connect` and `Tls` separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Resolving the host name.
    Dns,
    /// Establishing the transport. For a client that can't separate them,
    /// this covers TCP and TLS together and `Tls` stays unset.
    Connect,
    /// The TLS handshake alone.
    Tls,
    /// Writing the request, body included.
    Send,
    /// Waiting for the response head — time to first byte.
    Wait,
    /// Reading the response body.
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    Info,
    Warn,
    Error,
}

/// One line of the request log, stamped with how long into the request it
/// happened rather than with a wall clock: the interesting question is
/// always "how far in", not "at what time of day".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub at_ms: u64,
    pub level: TraceLevel,
    pub message: String,
}

/// Where the time went, plus what happened along the way.
///
/// Every phase is `Option` on purpose, and the distinction carries meaning:
/// `None` is "this phase did not happen or could not be measured" (a request
/// on a pooled connection has no DNS or connect phase at all), while
/// `Some(0)` is "it happened and took under a millisecond". A UI must not
/// collapse the two.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub dns_ms: Option<u64>,
    pub connect_ms: Option<u64>,
    pub tls_ms: Option<u64>,
    pub send_ms: Option<u64>,
    pub wait_ms: Option<u64>,
    pub download_ms: Option<u64>,
    /// Wall time of the whole attempt, set when the recorder is finished.
    /// Unlike the phases this is always known — including for an attempt
    /// that failed part way.
    pub total_ms: u64,
    pub reused_connection: Option<bool>,
    pub remote_addr: Option<String>,
    pub events: Vec<TraceEvent>,
}

/// What an executor writes its timings and log into.
///
/// Deliberately passed in by the caller rather than returned inside
/// `ExecutionOutcome`: a request that fails has no outcome, and a timeline
/// showing that thirty seconds went into waiting is exactly what explains
/// the failure. The caller keeps the recorder and can finish it on either
/// path.
pub struct TraceRecorder {
    started: Instant,
    trace: ExecutionTrace,
}

impl TraceRecorder {
    pub fn start() -> Self {
        TraceRecorder {
            started: Instant::now(),
            trace: ExecutionTrace::default(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn at_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Records how long a phase took. Called after the fact, so an executor
    /// that measures a phase it can't attribute (TCP and TLS as one) simply
    /// leaves the finer phase unset.
    pub fn phase(&mut self, phase: Phase, duration: Duration) {
        let millis = duration.as_millis() as u64;
        let slot = match phase {
            Phase::Dns => &mut self.trace.dns_ms,
            Phase::Connect => &mut self.trace.connect_ms,
            Phase::Tls => &mut self.trace.tls_ms,
            Phase::Send => &mut self.trace.send_ms,
            Phase::Wait => &mut self.trace.wait_ms,
            Phase::Download => &mut self.trace.download_ms,
        };
        *slot = Some(millis);
    }

    pub fn event(&mut self, level: TraceLevel, message: impl Into<String>) {
        let at_ms = self.at_ms();
        self.trace.events.push(TraceEvent {
            at_ms,
            level,
            message: message.into(),
        });
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.event(TraceLevel::Info, message);
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        self.event(TraceLevel::Warn, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.event(TraceLevel::Error, message);
    }

    pub fn set_remote_addr(&mut self, addr: impl Into<String>) {
        self.trace.remote_addr = Some(addr.into());
    }

    pub fn set_reused_connection(&mut self, reused: bool) {
        self.trace.reused_connection = Some(reused);
    }

    /// Stamps the total and hands over the collected trace. Called on both
    /// the success and the failure path.
    pub fn finish(mut self) -> ExecutionTrace {
        self.trace.total_ms = self.at_ms();
        self.trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_phases_events_and_a_total() {
        let mut recorder = TraceRecorder::start();
        recorder.info("отправка запроса");
        recorder.phase(Phase::Wait, Duration::from_millis(120));
        recorder.phase(Phase::Download, Duration::from_millis(7));
        recorder.set_remote_addr("93.184.216.34:443");
        recorder.warn("переменная не подставлена");

        let trace = recorder.finish();

        assert_eq!(trace.wait_ms, Some(120));
        assert_eq!(trace.download_ms, Some(7));
        assert_eq!(trace.remote_addr.as_deref(), Some("93.184.216.34:443"));
        assert_eq!(trace.events.len(), 2);
        assert_eq!(trace.events[1].level, TraceLevel::Warn);
        // Phases are reported by the executor and need not add up to the
        // total, which is measured independently.
        assert!(trace.total_ms < 1000);
    }

    #[test]
    fn unmeasured_phases_stay_none_and_are_not_zero() {
        let mut recorder = TraceRecorder::start();
        recorder.phase(Phase::Connect, Duration::from_micros(200));
        let trace = recorder.finish();

        // A phase that happened but was faster than a millisecond, versus
        // phases that never happened at all: the difference has to survive.
        assert_eq!(trace.connect_ms, Some(0));
        assert_eq!(trace.dns_ms, None);
        assert_eq!(trace.tls_ms, None);
        assert_eq!(trace.reused_connection, None);
    }
}

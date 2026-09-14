//! Process-isolated local algorithm protocol. No shell, paths supplied only by Host.
use crate::{executor::Values, pool::MAX_PAYLOAD_BYTES};
use serde_json::{Value, json};
use std::io::Read;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub struct PluginWorker {
    launch: [std::path::PathBuf; 4],
    expected: Vec<String>,
    restart_required: bool,
    child: Child,
    input: mpsc::SyncSender<Vec<u8>>,
    output: mpsc::Receiver<Result<Value, String>>,
}
impl PluginWorker {
    pub fn start(
        python: &Path,
        packages: &Path,
        script: &Path,
        model: &Path,
    ) -> Result<Self, String> {
        let mut child = Command::new(python)
            .arg("-B")
            .arg("-s")
            .arg("-u")
            .arg(script)
            .env("PYTHONPATH", packages)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env_remove("PYTHONHOME")
            .env("PYTHONNOUSERSITE", "1")
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .env("JUEMING_MODEL_DIR", model)
            .env("TOKENIZERS_PARALLELISM", "false")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("plugin spawn failed: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("worker stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("worker stdout unavailable")?;
        let (tx, rx) = mpsc::sync_channel(1);
        let (input, requests) = mpsc::sync_channel::<Vec<u8>>(1);
        let failures = tx.clone();
        std::thread::spawn(move || {
            while let Ok(bytes) = requests.recv() {
                if let Err(error) = stdin.write_all(&bytes).and_then(|_| stdin.flush()) {
                    let _ = failures.send(Err(format!("plugin input disconnected: {error}")));
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                // take() limits allocation even for a hostile process without newlines.
                let mut frame = Vec::new();
                let result = std::io::Read::by_ref(&mut reader)
                    .take((MAX_PAYLOAD_BYTES + 1) as u64)
                    .read_until(b'\n', &mut frame);
                let value = match result {
                    Ok(0) => Err("plugin closed its output".into()),
                    Ok(_) if frame.len() > MAX_PAYLOAD_BYTES || frame.last() != Some(&b'\n') => {
                        Err("plugin frame exceeds protocol limit".into())
                    }
                    Ok(_) => serde_json::from_slice(&frame).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let failed = value.is_err();
                if tx.send(value).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            launch: [python.into(), packages.into(), script.into(), model.into()],
            expected: Vec::new(),
            restart_required: false,
            child,
            input,
            output: rx,
        })
    }
    pub fn call(&mut self, mut request: Value, cancelled: &AtomicBool) -> Result<Value, String> {
        let id = uuid::Uuid::now_v7().to_string();
        request["request_id"] = json!(id);
        let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        if bytes.len() >= MAX_PAYLOAD_BYTES {
            return Err("plugin input batch too large".into());
        }
        bytes.push(b'\n');
        // A blocked child stdin must not prevent cancellation or timeout.
        self.input.try_send(bytes).map_err(|e| e.to_string())?;
        let start = Instant::now();
        loop {
            if cancelled.load(Ordering::Acquire) || start.elapsed() > Duration::from_secs(300) {
                self.restart_required = true;
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err(if cancelled.load(Ordering::Acquire) {
                    "cancelled"
                } else {
                    "plugin request timed out"
                }
                .into());
            }
            match self.output.recv_timeout(Duration::from_millis(100)) {
                Ok(response) => {
                    let response = response?;
                    if response.get("request_id").and_then(Value::as_str) != Some(&id) {
                        return Err("plugin response identity mismatch".into());
                    }
                    if let Some(error) = response.get("error") {
                        return Err(error.as_str().unwrap_or("plugin failed").into());
                    }
                    return response
                        .get("data")
                        .cloned()
                        .ok_or("plugin omitted response data".into());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("plugin output disconnected".into()),
            }
        }
    }
    pub fn handshake(&mut self, expected: &[String], cancelled: &AtomicBool) -> Result<(), String> {
        let result = self.call(json!({"method":"hello"}), cancelled)?;
        if result.get("protocol_version") != Some(&json!(1))
            || result.get("operators") != Some(&json!(expected))
        {
            return Err("incompatible plugin handshake".into());
        }
        self.expected = expected.to_vec();
        Ok(())
    }
    pub fn execute(
        &mut self,
        operator: &str,
        inputs: Values,
        config: &Value,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String> {
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        if self.restart_required {
            let mut replacement = Self::start(
                &self.launch[0],
                &self.launch[1],
                &self.launch[2],
                &self.launch[3],
            )?;
            replacement.handshake(&self.expected, cancelled)?;
            *self = replacement;
        }
        serde_json::from_value(self.call(
            json!({"method":"execute","operator_id":operator,"inputs":inputs,"config":config}),
            cancelled,
        )?)
        .map_err(|e| e.to_string())
    }
}
impl Drop for PluginWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

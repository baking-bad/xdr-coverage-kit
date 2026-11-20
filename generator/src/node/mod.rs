//! A local standalone stellar-core node that the generator drives.

use crate::codec::LIMITS;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread::sleep,
    time::Duration,
};
use stellar_xdr::{
    Frame, LedgerCloseMeta, Limited, ReadXdr, TransactionEnvelope,
    TransactionResult, WriteXdr,
};
use tempfile::TempDir;

/// The settings of the node. `passphrase`, `protocol` and `version` decide
/// the bytes of the corpus.
pub struct NodeConfig {
    /// The stellar-core program: a name on `PATH` or a path.
    pub binary: PathBuf,
    /// The local HTTP port of the node.
    pub port: u16,
    /// The network passphrase. It enters every transaction hash.
    pub passphrase: String,
    /// The protocol that the node upgrades to before the first transaction.
    pub protocol: u32,
    /// The release of stellar-core that produces the corpus.
    pub version: &'static str,
}

impl NodeConfig {
    /// The settings of a standalone network.
    pub fn standalone(binary: PathBuf) -> NodeConfig {
        NodeConfig {
            binary,
            port: 11626,
            passphrase: "Standalone Network ; February 2017".to_string(),
            protocol: 30,
            version: "30.0.0",
        }
    }

    pub fn network_id(&self) -> [u8; 32] {
        Sha256::digest(&self.passphrase).into()
    }
}

/// Fails unless the stellar-core program is the release of the settings.
/// Another release may close other bytes for the same scenario.
pub fn require_version(config: &NodeConfig) -> Result<()> {
    let output = Command::new(&config.binary)
        .arg("version")
        .output()
        .with_context(|| {
            format!(
                "run {}: install stellar-core {} on PATH, or name it with \
                 --stellar-core",
                config.binary.display(),
                config.version
            )
        })?;
    ensure!(
        output.status.success(),
        "'{} version' failed with {}",
        config.binary.display(),
        output.status
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text
        .lines()
        .find_map(|line| line.strip_prefix("stellar-core "))
        .and_then(|rest| rest.split_whitespace().next())
        .with_context(|| {
            format!(
                "'{} version' printed no line 'stellar-core <release>'",
                config.binary.display()
            )
        })?;
    ensure!(
        version == config.version,
        "stellar-core is release {version}, and generate needs {}: name that \
         release with --stellar-core",
        config.version
    );
    Ok(())
}

// NOTE: with `RUN_STANDALONE` and `MANUAL_CLOSE` in `core.cfg`, stellar-core
// runs on a virtual clock. An idle node jumps that clock to its next timer,
// so the clock gains one to two days for each real second. stellar-core
// refuses a close time behind that clock. So the ledgers stand 91 days
// apart, which leaves a scenario at least half a minute between two closes.
// The close times are fixed, so they do not depend on the time of the run.
/// The close time of the first ledger, in seconds since 1970.
const FIRST_CLOSE_TIME: u64 = 1_000_000_000;
/// The seconds between the close times of two ledgers.
const CLOSE_TIME_STEP: u64 = 91 * 86_400;

/// The sequence number of the first ledger that the node closes.
// NOTE: a new network starts at ledger 1.
const FIRST_CLOSED_LEDGER: u64 = 2;

/// The answer of stellar-core to a submitted envelope.
#[derive(Deserialize)]
struct Submitted {
    status: String,
    error: Option<String>,
}

/// A running node. Dropping it stops the process and removes its files.
pub struct Node {
    port: u16,
    process: Child,
    dir: TempDir,
    closed: u64,
}

impl Node {
    /// Starts a new network of one node and upgrades it to the protocol
    /// version of the settings, which closes the first ledger. Fails when
    /// another process already answers on the port.
    pub fn start(config: &NodeConfig) -> Result<Node> {
        let probe = http().get(&url(config.port, "info")).call();
        if let Ok(_) | Err(ureq::Error::StatusCode(_)) = probe {
            bail!(
                "port {} already answers: stop the process that listens on \
                 it, for example another stellar-core",
                config.port
            );
        }
        let dir = tempfile::Builder::new()
            .prefix("xdr-coverage-kit-")
            .tempdir()
            .context("create the node directory")?;
        fs::write(dir.path().join("core.cfg"), core_cfg(config))
            .context("write core.cfg")?;

        let new_db = core(config, dir.path(), "new-db")
            .output()
            .with_context(|| format!("run {}", config.binary.display()))?;
        ensure!(
            new_db.status.success(),
            "'{} new-db' failed with {}: {}",
            config.binary.display(),
            new_db.status,
            String::from_utf8_lossy(&new_db.stdout)
        );
        let process = core(config, dir.path(), "run")
            .stdout(Stdio::null())
            .spawn()
            .context("start stellar-core")?;
        let mut node = Node {
            port: config.port,
            process,
            dir,
            closed: 0,
        };
        node.schedule_upgrade(config.protocol)?;
        node.close()?;
        Ok(node)
    }

    /// Queues a transaction. The next `close` applies it. Fails unless
    /// stellar-core answers `PENDING`.
    pub fn submit(&self, envelope: &TransactionEnvelope) -> Result<()> {
        let answer: Submitted = http()
            .get(&url(self.port, "tx"))
            .query("blob", &envelope.to_xdr_base64(LIMITS)?)
            .call()
            .context("submit the envelope")?
            .body_mut()
            .read_json()?;
        if answer.status == "PENDING" {
            return Ok(());
        }
        // NOTE: on a refusal `error` holds a `TransactionResult` in base64.
        let reason = answer
            .error
            .and_then(|error| {
                TransactionResult::from_xdr_base64(error, LIMITS).ok()
            })
            .map(|result| serde_json::to_string(&result.result))
            .transpose()?;
        bail!(
            "stellar-core answered {} to the envelope: {}",
            answer.status,
            reason.as_deref().unwrap_or("no reason")
        )
    }

    /// Closes the next ledger at its fixed time, with the queued transactions.
    pub fn close(&mut self) -> Result<()> {
        let ledger = FIRST_CLOSED_LEDGER + self.closed;
        let time = FIRST_CLOSE_TIME + self.closed * CLOSE_TIME_STEP;
        let answer = self
            .get(&format!("manualclose?ledgerSeq={ledger}&closeTime={time}"))?;
        ensure!(
            answer.contains("Manually closed ledger"),
            "stellar-core did not close ledger {ledger}: {answer}"
        );
        self.closed += 1;
        Ok(())
    }

    /// The `LedgerCloseMeta` of the last closed ledger.
    pub fn last_ledger(&self) -> Result<LedgerCloseMeta> {
        let stream = File::open(self.dir.path().join("meta.xdr"))
            .context("open meta.xdr")?;
        last_frame(stream)
    }

    /// Waits for the node and schedules the upgrade to `protocol`.
    /// One request schedules the upgrade to any version, and the next
    /// close applies it.
    fn schedule_upgrade(&mut self, protocol: u32) -> Result<()> {
        let request = format!(
            "upgrades?mode=set&upgradetime=1970-01-01T00:00:00Z\
             &protocolversion={protocol}"
        );
        for _ in 0..1000 {
            if let Ok(Some(status)) = self.process.try_wait() {
                bail!("stellar-core stopped: {status}");
            }
            // NOTE: the HTTP port answers before the node can take
            // an upgrade, and until then the answer is "Core is booting".
            match self.get(&request) {
                Ok(answer) if answer.contains("booting") => {}
                Ok(answer) if answer.contains("error") => {
                    bail!("stellar-core refused the upgrade: {answer}")
                }
                Ok(_) => return Ok(()),
                Err(_) => {}
            }
            sleep(Duration::from_millis(10));
        }
        bail!(
            "stellar-core did not accept the upgrade to protocol {protocol} \
             after 1000 requests"
        )
    }

    fn get(&self, path: &str) -> Result<String> {
        let answer = http()
            .get(&url(self.port, path))
            .call()
            .with_context(|| format!("request {path}"))?;
        Ok(answer.into_body().read_to_string()?)
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// An HTTP client that fails a request after ten seconds, so a node that
/// accepts a request and does not answer stops the run.
fn http() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into()
}

fn url(port: u16, path: &str) -> String {
    format!("http://localhost:{port}/{path}")
}

fn core(config: &NodeConfig, dir: &Path, command: &str) -> Command {
    let mut core = Command::new(&config.binary);
    core.args([command, "--conf", "core.cfg"])
        .current_dir(dir)
        .stderr(Stdio::null());
    core
}

/// The text of `core.cfg` with its placeholders filled.
fn core_cfg(config: &NodeConfig) -> String {
    include_str!("core.cfg")
        .replace("{port}", &config.port.to_string())
        .replace("{passphrase}", &config.passphrase)
}

/// The last value of `METADATA_OUTPUT_STREAM`, a stream of XDR frames.
fn last_frame(stream: impl Read) -> Result<LedgerCloseMeta> {
    let mut stream = Limited::new(stream, LIMITS);
    let ledgers = Frame::<LedgerCloseMeta>::read_xdr_iter(&mut stream)
        .collect::<Result<Vec<_>, _>>()
        .context("read meta.xdr")?;
    let Frame(last) = ledgers
        .into_iter()
        .last()
        .context("meta.xdr holds no ledger")?;
    Ok(last)
}

//! An Intel Mac under Linux keeps what it knows about power coming in (which port, what was
//! negotiated) in its SMC. The applesmc driver publishes any key through four files: write an
//! index to key_at_index, then key_at_index_name and key_at_index_data are that key's. Only
//! root may write the index unless the machine says otherwise, so every read here may fail,
//! and the collectors then report what sysfs alone knows.

use super::sysfs;
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

pub struct Smc {
    dir: String,
    /// A recorded tree (tests/fixtures) has no driver behind it: its keys are files, keys/<NAME>.
    recorded: bool,
    count: usize,
}

// Where each key sits. The SMC's keys do not move while the machine is up.
static INDEX_OF: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);

impl Smc {
    /// None when the machine has no SMC, or will not let this user pick a key.
    pub fn open(sys: &str) -> Option<Smc> {
        let dir = format!("{sys}/devices/platform/applesmc.768");
        if fs::metadata(format!("{dir}/keys")).is_ok_and(|keys| keys.is_dir()) {
            return Some(Smc { dir, recorded: true, count: 0 });
        }
        let count = sysfs::int(&format!("{dir}/key_count")).filter(|count| *count > 0)? as usize;
        fs::write(format!("{dir}/key_at_index"), "0").ok()?;
        Some(Smc { dir, recorded: false, count })
    }

    pub fn bytes(&self, name: &str) -> Option<Vec<u8>> {
        if self.recorded {
            return fs::read(format!("{}/keys/{name}", self.dir)).ok();
        }
        let index = self.index_of(name)?;
        self.select(index)?;
        let data = fs::read(format!("{}/key_at_index_data", self.dir)).ok()?;
        // Another reader may have moved the index between the two steps; then the data is not ours.
        (self.selected_name()? == name).then_some(data)
    }

    pub fn u8(&self, name: &str) -> Option<u8> {
        self.bytes(name)?.first().copied()
    }

    /// The SMC's integers are big-endian.
    pub fn u16(&self, name: &str) -> Option<u16> {
        let bytes = self.bytes(name)?;
        Some(u16::from_be_bytes([*bytes.first()?, *bytes.get(1)?]))
    }

    pub fn string(&self, name: &str) -> Option<String> {
        let bytes = self.bytes(name)?;
        let text: Vec<u8> = bytes.into_iter().take_while(|&byte| byte != 0).collect();
        let text = String::from_utf8_lossy(&text).trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    fn select(&self, index: usize) -> Option<()> {
        fs::write(format!("{}/key_at_index", self.dir), index.to_string()).ok()
    }

    fn selected_name(&self) -> Option<String> {
        sysfs::string(&format!("{}/key_at_index_name", self.dir))
    }

    // The driver lists the keys sorted by name, so a name is found by halving, as the driver
    // itself does, and remembered.
    fn index_of(&self, name: &str) -> Option<usize> {
        let mut known = INDEX_OF.lock().ok()?;
        let known = known.get_or_insert_with(HashMap::new);
        if let Some(index) = known.get(name) {
            return Some(*index);
        }
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let middle = (low + high) / 2;
            self.select(middle)?;
            let found = self.selected_name()?;
            match found.as_bytes().cmp(name.as_bytes()) {
                std::cmp::Ordering::Equal => {
                    known.insert(name.to_string(), middle);
                    return Some(middle);
                }
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
            }
        }
        None
    }
}

/// One port's power contract, as the SMC holds it.
pub struct Contract {
    pub port: u8,
    pub millivolts: u16,
    pub milliamps: u16,
}

impl Smc {
    /// How many ports can take power in (AC-N), and the contract on each that has one.
    pub fn contracts(&self) -> (u8, Vec<Contract>) {
        let ports = self.u8("AC-N").filter(|ports| (1..=8).contains(ports)).unwrap_or(0);
        let contracts = (1..=ports)
            .filter_map(|port| {
                let millivolts = self.u16(&format!("D{port}VR")).filter(|&value| value > 0)?;
                let milliamps = self.u16(&format!("D{port}IR")).filter(|&value| value > 0)?;
                Some(Contract { port, millivolts, milliamps })
            })
            .collect();
        (ports, contracts)
    }

    /// The port the machine is drawing from: the one the SMC names (AC-W) when that port holds
    /// a contract, else the first that does. A dock on two ports puts a contract on both.
    pub fn feeding<'a>(&self, contracts: &'a [Contract]) -> Option<&'a Contract> {
        let winner = self.u8("AC-W").unwrap_or(0);
        contracts.iter().find(|contract| contract.port == winner).or(contracts.first())
    }
}

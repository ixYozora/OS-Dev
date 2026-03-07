use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::AtomicUsize;
use usrlib::spinlock::Spinlock;
use super::vma::VMA;

static PROCESSES: Spinlock<BTreeMap<usize, Process>> = Spinlock::new(BTreeMap::new());
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
pub struct Process {
    id: usize,
    name: String,
    vmas: Vec<VMA>,
}

impl Process {
    pub fn new(name: &str) -> Self {
        let pid = NEXT_PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        Process { id: pid, name: String::from(name), vmas: Vec::new() }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
}

pub fn add_process(process: Process) {
    PROCESSES.lock().insert(process.id, process);
}

pub fn remove_process(process_id: usize) {
    PROCESSES.lock().remove(&process_id);
}

pub fn get_app_name(process_id: usize) -> Option<String> {
    PROCESSES.lock().get(&process_id).map(|p| p.name.clone())
}

pub fn add_vma(process_id: usize, vma: VMA) -> Result<(), &'static str> {
    let mut procs = PROCESSES.lock();
    let process = procs.get_mut(&process_id).ok_or("Process not found")?;

    for existing in process.vmas.iter() {
        if existing.overlaps(&vma) {
            return Err("VMA overlaps with existing VMA");
        }
    }

    process.vmas.push(vma);
    Ok(())
}

pub fn dump_vmas(process_id: usize) {
    let procs = PROCESSES.lock();
    if let Some(process) = procs.get(&process_id) {
        kprintln!("VMAs for process {} ({}):", process.id, process.name);
        for vma in process.vmas.iter() {
            kprintln!("  {:?}", vma);
        }
    } else {
        kprintln!("Process {} not found", process_id);
    }
}
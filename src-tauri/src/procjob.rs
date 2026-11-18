//! Processos filhos presos a um Job Object com KILL_ON_JOB_CLOSE: se o Spotron fechar ou travar,
//! o Windows mata o Edge do player e o CamillaDSP junto (nada fica órfão).

use std::process::{Child, Command};

/// Sem janela de console para os filhos (o app de release não tem console).
pub fn hide_console(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub struct Managed {
    pub child: Child,
    #[cfg(windows)]
    #[allow(dead_code)] // mantido só para o job continuar aberto
    job: JobHandle,
}

impl Managed {
    pub fn spawn(cmd: &mut Command) -> Result<Self, String> {
        let child = cmd.spawn().map_err(|e| e.to_string())?;
        #[cfg(windows)]
        let job = job_for(&child)?;
        Ok(Self {
            child,
            #[cfg(windows)]
            job,
        })
    }

    pub fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Managed {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(windows)]
struct JobHandle(windows_sys::Win32::Foundation::HANDLE);
// O handle só é mantido aberto e fechado no Drop.
#[cfg(windows)]
unsafe impl Send for JobHandle {}

#[cfg(windows)]
impl Drop for JobHandle {
    fn drop(&mut self) {
        // Fechar o último handle do job mata todos os processos dele.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(windows)]
fn job_for(child: &Child) -> Result<JobHandle, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::*;
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err("CreateJobObjectW falhou".into());
        }
        let handle = JobHandle(job);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0 {
            return Err("SetInformationJobObject falhou".into());
        }
        if AssignProcessToJobObject(job, child.as_raw_handle() as _) == 0 {
            return Err("AssignProcessToJobObject falhou".into());
        }
        Ok(handle)
    }
}

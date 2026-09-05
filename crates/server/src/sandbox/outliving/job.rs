//! The Job Object a process Verkstead started is put in, so that it goes when
//! Verkstead does.
//!
//! **What `--die-with-parent` is on Linux**, said in the one vocabulary this
//! platform has for it. A Job configured to kill everything left in it when the
//! last handle to it closes is a promise kept from outside the process: a
//! server that exits, is killed, or crashes closes its handles either way, and
//! what was inside the Job goes with them. Nothing has to run at that moment,
//! which is the whole reason the Mac needs a keeper and this does not — see
//! [`super`], where that comparison is made.
//!
//! **Everything a process in a Job starts is in the Job too**, which is what
//! makes one a whole tree rather than one process. That is what a session's
//! agent needs — a build it left running is as much the session's as the agent
//! is — and it is what the Compile Server needs for the same reason.
//!
//! Two things hold one. A session's terminal makes it around the process it
//! starts on a pseudoconsole — see [`crate::terminal`] — and the Compile
//! Server is put in one by [`super::held`], there being no terminal under it to
//! do it. The type is here rather than in either, because it is the platform's
//! answer to a question [`super`] is the module about.

use std::ffi::c_void;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

/// One Job, closed when it is let go of — which is what kills what is in it.
///
/// A handle is a pointer as far as the bindings are concerned and therefore
/// neither `Send` nor `Sync` by itself, and it is both in fact: it is a number
/// the kernel looks up in a table this whole process shares, and nothing about
/// which thread holds it means anything.
#[derive(Debug)]
pub(crate) struct Job(HANDLE);

unsafe impl Send for Job {}
unsafe impl Sync for Job {}

impl Job {
    /// A Job that kills everything left in it when the last handle to it
    /// closes, which is the whole of what this type is for.
    pub(crate) fn killing_everything_in_it() -> io::Result<Job> {
        // Safety: an unnamed Job with the default security descriptor, which is
        // what both of these nulls say.
        let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };

        if job.is_null() {
            return Err(io::Error::last_os_error());
        }

        let job = Job(job);

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        // Safety: the structure is the one the class is documented to take, and
        // what is passed as its length is its length.
        let told = unsafe {
            SetInformationJobObject(
                job.handle(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast::<c_void>(),
                u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                    .unwrap_or(u32::MAX),
            )
        };

        if told == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(job)
    }

    /// The handle itself, for the calls that take one.
    pub(crate) fn handle(&self) -> HANDLE {
        self.0
    }

    /// Put `process` in it. Everything that process goes on to start is in it
    /// too, which is what makes the Job a whole tree.
    pub(crate) fn take(&self, process: HANDLE) -> io::Result<()> {
        // Safety: both handles are the caller's own and outlive the call.
        if unsafe { AssignProcessToJobObject(self.handle(), process) } == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// And end everything in it now, saying it exited `code` — which is the
    /// same kill the close makes, asked for rather than waited for.
    pub(crate) fn terminate(&self, code: u32) -> io::Result<()> {
        // Safety: the handle is this Job's own and outlives the call.
        if unsafe { TerminateJobObject(self.handle(), code) } == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

impl Drop for Job {
    /// Which is what kills what is inside it — see this module's own
    /// documentation.
    fn drop(&mut self) {
        // Safety: the handle is this Job's own and is not used again.
        unsafe { CloseHandle(self.0) };
    }
}

//! Definition of built-in processes.

use crate::process::{spawn, wait};


/// The 'init' builtin process.
pub extern "C" fn init() {
    spawn(shell, "[shell]");
    loop {
        wait();
    }
}


/// The 'sh' builtin process.
pub extern "C" fn shell() {

    

}

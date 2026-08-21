//! Rendering a suite report, a check report, or a doctor diagnosis for people
//! and for machines, and their exit codes.
//!
//! Responsibilities: [`Report`], [`render_human`], [`render_check_human`],
//! [`render_doctor_human`], [`render_json`], [`render_wiring_human`],
//! [`exit_code`], [`check_exit_code`], [`doctor_exit_code`].

mod render;

pub use render::{
    Report, check_exit_code, doctor_exit_code, exit_code, render_check_human, render_doctor_human,
    render_human, render_json, render_wiring_human,
};

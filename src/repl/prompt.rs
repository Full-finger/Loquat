//! Prompt formatting for REPL

use crate::repl::context::ReplContext;
use colored::Colorize;

/// Generate the REPL prompt
pub fn generate_prompt(ctx: &ReplContext) -> String {
    let env = ctx.config.general.environment.clone();
    
    // Check if there are errors
    let has_errors = check_for_errors(ctx);
    
    // Format the prompt
    if has_errors {
        format!("loquat[{}|{}]> ", env, "!".red())
    } else {
        format!("loquat[{}]> ", env.bright_green())
    }
}

/// Check if there are any errors in the system
fn check_for_errors(ctx: &ReplContext) -> bool {
    // Check plugins for errors
    if let Some(plugin_manager) = &ctx.plugin_manager {
        let plugins = plugin_manager.list_plugin_infos();
        if plugins.iter().any(|p| {
            matches!(p.status, crate::plugins::types::PluginStatus::Error { .. })
        }) {
            return true;
        }
    }
    
    // Note: We can't check adapters or engine asynchronously in prompt generation
    // These would require async/await which isn't available in sync functions
    
    false
}

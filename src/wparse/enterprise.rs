use wp_enterprise::plugin::{build_skyeye_processor, pass::build_pass_processor};
use wpl::register_wpl_pipe;

pub fn register() {
    register_wpl_pipe!("ent/pass", build_pass_processor);
    register_wpl_pipe!("qax/skyeye", build_skyeye_processor);
}

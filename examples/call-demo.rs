use sixg_scenario::{CallDemoScenario, CallDemoError};

fn main() -> Result<(), CallDemoError> {
    let mut scenario = CallDemoScenario::new();
    let report = scenario.run()?;

    report.print_summary();
    report.write_json(std::path::Path::new(
        "validation-output/call-demo/trace.json",
    ))?;

    if !report.passed() {
        std::process::exit(1);
    }

    Ok(())
}

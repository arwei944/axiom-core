use axiom_kernel::witness::{TransitionOutcome, WitnessBuilder, WitnessKernel};
use axiom_kernel::KernelResult;

fn main() -> KernelResult<()> {
    let kernel = WitnessKernel::new();

    let summary = "cell created";
    let _builder = WitnessBuilder::new()
        .summary(summary)
        .outcome(TransitionOutcome::Success);

    println!("=== guard-audit example ===");
    println!("witness summary: {}", summary);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut witnesses = Vec::new();
        
        for i in 0..2 {
            let witness = WitnessBuilder::new()
                .summary(format!("cell transition {}", i))
                .outcome(TransitionOutcome::Success);
            witnesses.push(witness);
        }

        println!("recorded {} witness builders", witnesses.len());

        let verified = kernel.verify_chain().await;
        println!("witness chain verified: {:?}", verified.is_ok());
    });

    println!("guard-audit example completed");
    Ok(())
}

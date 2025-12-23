//! Basic usage example for Loquat framework
//! Demonstrates AOP + logging integration

use loquat::*;
use std::sync::Arc;

// Example service that we want to apply AOP to
struct CalculatorService {
    name: String,
}

impl CalculatorService {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    fn add(&self, a: i32, b: i32) -> Result<i32> {
        println!("Calculating {} + {}", a, b);
        Ok(a + b)
    }

    fn divide(&self, a: i32, b: i32) -> Result<i32> {
        if b == 0 {
            return Err(loquat::errors::AopError::ExecutionFailed("Division by zero".to_string()).into());
        }
        println!("Calculating {} / {}", a, b);
        Ok(a / b)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a logger with text formatter and console writer
    let writer = Arc::new(loquat::logging::writers::ConsoleWriter::new());
    let formatter = Arc::new(loquat::logging::formatters::TextFormatter::detailed());
    let logger: Arc<dyn loquat::logging::traits::Logger> = 
        Arc::new(loquat::logging::logger::StructuredLogger::new(formatter, writer));

    // Create a calculator service
    let calculator = CalculatorService::new("MyCalculator");

    // Create an AOP proxy with logging and error tracking
    let proxy = AopProxyFactory::create_with_logging(calculator, Arc::clone(&logger));

    // Execute operations with AOP aspects applied
    println!("\n=== Testing Addition ===");
    let result = proxy.execute_with_aspects("add", |calc| {
        calc.add(10, 20)
    }).await?;
    println!("Result: {}", result);

    println!("\n=== Testing Division (Success) ===");
    let result = proxy.execute_with_aspects("divide", |calc| {
        calc.divide(20, 4)
    }).await?;
    println!("Result: {}", result);

    println!("\n=== Testing Division (Error) ===");
    let result = proxy.execute_with_aspects("divide", |calc| {
        calc.divide(10, 0)
    }).await;
    match result {
        Ok(r) => println!("Result: {}", r),
        Err(e) => println!("Error occurred (expected): {}", e),
    }

    println!("\n=== Testing with AOP Manager ===");
    let manager = AopFactory::create_with_logging(Arc::clone(&logger));
    
    let sum = manager.apply_aspects("manual_add", || {
        Ok(5 + 3)
    }).await?;
    println!("Manual calculation result: {}", sum);

    Ok(())
}

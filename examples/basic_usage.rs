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

    // Create AOP proxy without factory (direct implementation)
    use loquat::aop::traits::{Aspect, JoinPoint, ExecutionResult};
    use loquat::logging::traits::{LogContext, LogLevel};
    
    struct LoggingAspect {
        logger: Arc<dyn loquat::logging::traits::Logger>,
    }

    impl Aspect for LoggingAspect {
        async fn before(&self, _join_point: &JoinPoint) -> ExecutionResult<()> {
            ExecutionResult::Continue(())
        }

        async fn after(&self, _join_point: &JoinPoint, result: &ExecutionResult<()>, context: &LogContext) -> ExecutionResult<()> {
            if let Err(e) = result {
                let mut new_context = context.clone();
                new_context.add("error", e.to_string());
                self.logger.log(LogLevel::Error, &format!("Operation failed: {}", e), &new_context);
            }
            ExecutionResult::Continue(())
        }
    }

    let logging_aspect = Arc::new(LoggingAspect { logger: logger.clone() });

    // Create proxy with the aspect
    let proxy = loquat::aop::proxy::AopProxy::new(calculator, logging_aspect);

    // Execute operations with AOP aspects applied
    println!("\n=== Testing Addition ===");
    let result = proxy.execute("add", |calc| {
        calc.add(10, 20)
    }).await?;
    println!("Result: {}", result);

    println!("\n=== Testing Division (Success) ===");
    let result = proxy.execute("divide", |calc| {
        calc.divide(20, 4)
    }).await?;
    println!("Result: {}", result);

    println!("\n=== Testing Division (Error) ===");
    let result = proxy.execute("divide", |calc| {
        calc.divide(10, 0)
    }).await?;
    println!("Result: {}", result);

    println!("\n=== Testing with AOP Manager ===");
    let manager = loquat::aop::factory::AopFactory::create_with_logging(logger.clone());
    
    let sum = manager.apply("manual_add", || {
        Ok(5 + 3)
    }).await?;
    println!("Manual calculation result: {}", sum);

    Ok(())
}

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
            return Err(loquat::errors::Error::Internal("Division by zero".to_string()));
        }
        println!("Calculating {} / {}", a, b);
        Ok(a / b)
    }
}

// Simple logging aspect for demonstration
struct LoggingAspect {
    logger: Arc<dyn loquat::logging::traits::Logger>,
}

#[async_trait::async_trait]
impl loquat::aop::traits::Aspect for LoggingAspect {
    async fn before(&self, operation: &str) -> loquat::errors::AopResult<()> {
        self.logger.log(
            loquat::logging::traits::LogLevel::Info,
            &format!("Starting operation: {}", operation),
            &loquat::logging::traits::LogContext::new()
        );
        Ok(())
    }

    async fn after(
        &self,
        operation: &str,
        result: &loquat::errors::AopResult<()>,
    ) -> loquat::errors::AopResult<()> {
        match result {
            Ok(_) => {
                self.logger.log(
                    loquat::logging::traits::LogLevel::Info,
                    &format!("Operation {} completed successfully", operation),
                    &loquat::logging::traits::LogContext::new()
                );
            }
            Err(e) => {
                self.logger.log(
                    loquat::logging::traits::LogLevel::Error,
                    &format!("Operation {} failed: {}", operation, e),
                    &loquat::logging::traits::LogContext::new()
                );
            }
        }
        Ok(())
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

    // Create AOP manager with logging
    let mut manager = loquat::aop::AopManager::new();
    let logging_aspect = Arc::new(LoggingAspect { logger: logger.clone() });
    manager.add_aspect(logging_aspect);

    // Test 1: Use AOP manager for operations
    println!("\n=== Testing with AOP Manager ===");
    
    let sum = manager.apply_aspects("add", || {
        calculator.add(10, 20)
    }).await?;
    println!("Addition result: {}", sum);

    let division = manager.apply_aspects("divide", || {
        calculator.divide(20, 4)
    }).await?;
    println!("Division result: {}", division);

    // Test 2: Test error handling
    println!("\n=== Testing Error Handling ===");
    match manager.apply_aspects("divide", || {
        calculator.divide(10, 0)
    }).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Division failed as expected: {}", e),
    }

    // Test 3: Create proxy for multiple operations
    println!("\n=== Testing with AOP Proxy ===");
    let proxy = manager.create_proxy(calculator);

    let sum = manager.apply_aspects("proxy_add", || {
        proxy.target().add(5, 3)
    }).await?;
    println!("Proxy addition result: {}", sum);

    println!("\n=== All tests completed successfully! ===");
    Ok(())
}

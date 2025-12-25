//! AOP Proxy implementation for applying aspects to objects

use crate::aop::traits::{Aspect, Proxyable};
use crate::errors::Result;

/// AOP Proxy that wraps a target object and applies aspects to its operations
#[derive(Debug)]
pub struct AopProxy<T> {
    target: T,
    aspects: Vec<std::sync::Arc<dyn Aspect>>,
}

impl<T> AopProxy<T> {
    pub fn new(target: T, aspects: Vec<std::sync::Arc<dyn Aspect>>) -> Self {
        Self { target, aspects }
    }

    pub fn target(&self) -> &T {
        &self.target
    }

    pub fn target_mut(&mut self) -> &mut T {
        &mut self.target
    }

    pub fn add_aspect(&mut self, aspect: std::sync::Arc<dyn Aspect>) {
        self.aspects.push(aspect);
    }

    pub fn aspects(&self) -> &[std::sync::Arc<dyn Aspect>] {
        &self.aspects
    }

    pub async fn execute_with_aspects<F, R>(&self, operation: &str, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Result<R> + Send,
        R: Send,
    {
        for aspect in &self.aspects {
            aspect.before(operation).await
                .map_err(|e| crate::errors::Error::Aop(e))?;
        }

        let result = f(&self.target);

        for aspect in &self.aspects {
            let unit_result: crate::aop::traits::AopResult<()> = match &result {
                Ok(_) => Ok(()),
                Err(e) => Err(crate::errors::AopError::ExecutionFailed(e.to_string())),
            };
            aspect.after(operation, &unit_result).await
                .map_err(|e| crate::errors::Error::Aop(e))?;
        }

        result
    }
}

impl<T: Clone> Clone for AopProxy<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            aspects: self.aspects.clone(),
        }
    }
}

impl<T> Proxyable for AopProxy<T>
where
    T: Send + Sync,
{
    type Output = T;

    fn execute_with_aspects<F, R>(&self, aspects: &[std::sync::Arc<dyn Aspect>], operation: &str, f: F) -> crate::aop::traits::AopResult<R>
    where
        F: FnOnce() -> crate::aop::traits::AopResult<R> + Send,
        R: Send,
    {
        let future = async {
            for aspect in aspects {
                aspect.before(operation).await?;
            }

            let result = f();

            for aspect in aspects {
                let unit_result: crate::aop::traits::AopResult<()> = match &result {
                    Ok(_) => Ok(()),
                    Err(e) => Err(crate::errors::AopError::ExecutionFailed(e.to_string())),
                };
                aspect.after(operation, &unit_result).await?;
            }

            result
        };

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(future)
        })
    }

    fn target(&self) -> &Self::Output {
        &self.target
    }
}

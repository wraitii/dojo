use std::{fmt::Display, str::FromStr};

use convert_case::{Case, Casing};
use starknet_crypto::FieldElement;

use super::class::ClassDiff;
use super::contract::ContractDiff;
use super::StateDiff;
use crate::manifest::{
    BaseManifest, DeploymentManifest, ManifestMethods, BASE_CONTRACT_NAME, WORLD_CONTRACT_NAME,
};

#[cfg(test)]
#[path = "world_test.rs"]
mod tests;

/// Represents the state differences between the local and remote worlds.
#[derive(Debug, Clone)]
pub struct WorldDiff {
    pub world: ContractDiff,
    pub base: ClassDiff,
    pub contracts: Vec<ContractDiff>,
    pub models: Vec<ClassDiff>,
}

impl WorldDiff {
    pub fn compute(local: BaseManifest, remote: Option<DeploymentManifest>) -> WorldDiff {
        let models = local
            .models
            .iter()
            .map(|model| ClassDiff {
                name: model.name.to_string(),
                local_class_hash: *model.inner.class_hash(),
                original_class_hash: *model.inner.original_class_hash(),
                remote_class_hash: remote.as_ref().and_then(|m| {
                    // Remote models are detected from events, where only the struct
                    // name (pascal case) is emitted.
                    // Local models uses the fully qualified name of the model,
                    // always in snake_case from cairo compiler.
                    let model_name = model
                        .name
                        .split("::")
                        .last()
                        .unwrap_or(&model.name)
                        .from_case(Case::Snake)
                        .to_case(Case::Pascal);
                    println!("Comparing name {} with {}", model_name, model.name);
                    m.models.iter().find(|e| e.name.to_lowercase() == model_name.to_lowercase()).map(|s| *s.inner.class_hash())
                }),
            })
            .collect::<Vec<_>>();

        let base = ClassDiff {
            name: BASE_CONTRACT_NAME.into(),
            local_class_hash: *local.base.inner.class_hash(),
            original_class_hash: remote.as_ref().map(|m| *m.base.inner.class_hash()).unwrap(),
            remote_class_hash: remote.as_ref().map(|m| *m.base.inner.class_hash()),
        };

        let contracts = local
            .contracts
            .iter()
            .map(|contract| {
                let base_class_hash = {
                    let class_hash = contract.inner.base_class_hash;
                    if class_hash != FieldElement::ZERO {
                        class_hash
                    } else {
                        base.original_class_hash
                    }
                };

                ContractDiff {
                    name: contract.name.to_string(),
                    local_class_hash: *contract.inner.class_hash(),
                    original_class_hash: *contract.inner.original_class_hash(),
                    base_class_hash,
                    remote_class_hash: remote.as_ref().and_then(|m| {
                        m.contracts
                            .iter()
                            .find(|r| r.inner.class_hash() == contract.inner.class_hash())
                            .map(|r| *r.inner.class_hash())
                    }),
                }
            })
            .collect::<Vec<_>>();


        let world = ContractDiff {
            name: WORLD_CONTRACT_NAME.into(),
            local_class_hash: *local.world.inner.class_hash(),
            original_class_hash: FieldElement::from_str("0x00099b08b2ff33750916e36b5e241b5d4a63e8d48862bf90a68fec2ff58a8de6").unwrap(),
            base_class_hash: base.original_class_hash,
            remote_class_hash: remote.as_ref().map(|m| *m.world.inner.class_hash()),
        };

        WorldDiff { world, base, contracts, models }
    }

    pub fn count_diffs(&self) -> usize {
        let mut count = 0;

        if !self.world.is_same() {
            count += 1;
        }

        count += self.models.iter().filter(|s| !s.is_same()).count();
        count += self.contracts.iter().filter(|s| !s.is_same()).count();
        count
    }
}

impl Display for WorldDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.world)?;

        for model in &self.models {
            writeln!(f, "{model}")?;
        }

        for contract in &self.contracts {
            writeln!(f, "{contract}")?;
        }

        Ok(())
    }
}

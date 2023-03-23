### netuid 3
```rust
Rho: u16 = 10;
Kappa: u16 = 32_767; // 0.5 = 65535/2 
MaxAllowedUids: u16 = 4_096;
Issuance: u64 = 0;
MinAllowedWeights: u16 = 1024;
EmissionValue: u16 = 1_000_000_000;
MaxWeightsLimit: u16 = 1_000; // 1000/2^16 = 0.015
Tempo: u16 = 99;
AdjustmentInterval: u16 = 100;
TargetRegistrationsPerInterval: u16 = 2;
ImmunityPeriod: u16 = 4_096;
ActivityCutoff: u16 = 5_000;
MaxRegistrationsPerBlock: u16 = 1;
PruningScore : u16 = u16::MAX;
DefaultTake: u16 = 11_796; // 18% honest number.
ServingRateLimit: u64 = 50; 
TxRateLimit: u64 = 1_000;
```
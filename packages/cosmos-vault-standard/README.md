# How to use Extensions

For example, if you want to use the Keeper and Lockup extensions as well as a
custom extension:

```rust
use custom_extension::{CustomExtension}
use vault_standard::{ExecuteMsg as VaultStandardExecuteMsg};

pub enum ExtensionExecuteMsg {
    Keeper(KeeperExecuteMsg),
    Lockup(LockupExecuteMsg),
    Custom(CustomExtension)
}

type ExecuteMsg = VaultStandardExecuteMsg<ExtensionExecuteMsg>;
```

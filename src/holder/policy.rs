/* src/holder/policy.rs */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnloadPolicy {
	/// Removable - config is removed from memory when source file is deleted.
	#[default]
	Removable,
	/// Persistent - config is retained in memory even if source file is deleted.
	Persistent,
}

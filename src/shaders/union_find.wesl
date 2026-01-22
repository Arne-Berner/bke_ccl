/// Used in the merge step



/// Find goes through the parent nodes recursively, until it finds the root node.
/// This works, because root points to itself. Usually this would be done in place,
/// but wgsl does not allow mutable function parameters.


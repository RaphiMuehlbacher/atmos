use crate::error::CompilerError;
use crate::extension::SourceSpanExt;
use crate::parser::ast::{AstNode, Ident, PathSegment};
use crate::resolver::modules::{Binding, ImportId, ModuleId};
use crate::resolver::ResolverError;
use crate::Resolver;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportResult {
    Resolved,
    Unresolved,
    Indeterminate,
}

pub struct ImportResolver<'a, 'r> {
    r: &'a mut Resolver<'r>,
}

impl<'a, 'r> ImportResolver<'a, 'r> {
    pub fn new(resolver: &'a mut Resolver<'r>) -> Self {
        Self { r: resolver }
    }

    pub fn resolve(&mut self) {
        let mut unresolved_imports = std::mem::take(&mut self.r.unresolved_imports);

        loop {
            let mut made_progress = false;
            let mut still_unresolved = Vec::new();

            for import_id in unresolved_imports {
                match self.resolve_import(import_id) {
                    ImportResult::Resolved => {
                        made_progress = true;
                        self.finalize_import(import_id);
                    }
                    ImportResult::Indeterminate => {
                        still_unresolved.push(import_id);
                    }
                    ImportResult::Unresolved => {
                        still_unresolved.push(import_id);
                    }
                }
            }

            unresolved_imports = still_unresolved;

            if !made_progress {
                break;
            }
        }

        for import_id in &unresolved_imports {
            self.report_unresolved_import(*import_id);
        }

        self.r.unresolved_imports = unresolved_imports;
    }

    fn resolve_import(&mut self, import_id: ImportId) -> ImportResult {
        let import = self.r.module_arena.get_import(import_id);
        let path = import.path.clone();
        let starting_module = import.parent_module;

        let (mut current_module, segment_start) = self.resolve_first_segment(&path.segments, starting_module);

        for segment in path.segments.iter().skip(segment_start) {
            let ident = &segment.node.ident.node;

            match self.resolve_ident_in_module(current_module, ident) {
                Some(binding) => match binding {
                    Binding::Module(module_id) => {
                        current_module = module_id;
                    }
                    Binding::Item(_) => {
                        let import = self.r.module_arena.get_import_mut(import_id);
                        import.resolved_binding = Some(binding);
                        return ImportResult::Resolved;
                    }
                    Binding::Import(other_import_id) => {
                        let other_import = self.r.module_arena.get_import(other_import_id);
                        match &other_import.resolved_binding {
                            Some(resolved) => match resolved {
                                Binding::Module(module_id) => {
                                    current_module = *module_id;
                                }
                                Binding::Item(_) => {
                                    let resolved = resolved.clone();
                                    let import = self.r.module_arena.get_import_mut(import_id);
                                    import.resolved_binding = Some(resolved);
                                    return ImportResult::Resolved;
                                }
                                Binding::Import(_) => {
                                    return ImportResult::Indeterminate;
                                }
                            },
                            None => {
                                return ImportResult::Indeterminate;
                            }
                        }
                    }
                },
                None => {
                    return ImportResult::Unresolved;
                }
            }
        }

        let import = self.r.module_arena.get_import_mut(import_id);
        import.resolved_binding = Some(Binding::Module(current_module));
        ImportResult::Resolved
    }

    fn resolve_first_segment(&self, segments: &[AstNode<PathSegment>], parent_module: ModuleId) -> (ModuleId, usize) {
        if segments.is_empty() {
            return (parent_module, 0);
        }

        let first_ident = &segments[0].node.ident.node;

        match first_ident.name.as_str() {
            "crate" => (self.r.module_arena.root_id(), 1),
            "super" => {
                let module = self.r.module_arena.get(parent_module);
                let parent = module.parent().unwrap_or(parent_module);

                let mut current = parent;
                let mut skip = 1;

                for segment in segments.iter().skip(1) {
                    if segment.node.ident.node.name == "super" {
                        let module = self.r.module_arena.get(current);
                        current = module.parent().unwrap_or(current);
                        skip += 1;
                    } else {
                        break;
                    }
                }

                (current, skip)
            }
            "self" => (parent_module, 1),
            _ => (parent_module, 0),
        }
    }

    fn resolve_ident_in_module(&self, module_id: ModuleId, ident: &Ident) -> Option<Binding> {
        let module = self.r.module_arena.get(module_id);
        module.get(ident).cloned()
    }

    fn finalize_import(&mut self, import_id: ImportId) {
        let import = self.r.module_arena.get_import(import_id);
        let parent_module = import.parent_module;

        let binding_name = import
            .path
            .segments
            .last()
            .map(|seg| seg.node.ident.node.clone())
            .expect("import path should have at least one segment");

        let resolved_binding = import
            .resolved_binding
            .clone()
            .expect("import should be resolved before finalizing");

        self.r
            .module_arena
            .define(parent_module, binding_name, resolved_binding);
    }

    fn report_unresolved_import(&self, import_id: ImportId) {
        let import = self.r.module_arena.get_import(import_id);

        let path_str = import
            .path
            .segments
            .iter()
            .map(|seg| seg.node.ident.node.name.clone())
            .collect::<Vec<_>>()
            .join("::");

        let span = import
            .path
            .segments
            .first()
            .unwrap()
            .span
            .to(import.path.segments.last().unwrap().span);

        self.r
            .session
            .push_error(CompilerError::ResolverError(ResolverError::UnresolvedPath {
                src: self.r.session.get_named_source(),
                span,
                path: path_str,
            }));
    }
}

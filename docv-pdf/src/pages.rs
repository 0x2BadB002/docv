use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    structures::{
        page::Page,
        root::pages_tree::{InheritableAttributes, PagesTreeNode},
    },
    types::IndirectReference,
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Iterator over pages in a PDF document's page tree.
///
/// The `Pages` struct provides an iterator that traverses the PDF page tree
/// structure, resolving indirect references and flattening the hierarchy
/// into a sequence of individual page objects.
///
/// # Page Tree Structure
/// PDF documents organize pages in a tree structure where:
/// - The root node is a `/Pages` object
/// - Intermediate nodes are also `/Pages` objects (containing `/Kids`)
/// - Leaf nodes are `/Page` objects
/// - Attributes can be inherited from parent pages nodes
///
/// This iterator performs a depth-first traversal of this tree structure.
///
/// # Usage
/// ```
/// use std::path::PathBuf;
/// use docv_pdf::Document;
///
/// let mut document = Document::from_path(&PathBuf::from("../example_files/report1.pdf")).unwrap();
/// for page in document.pages() {
///     let page = page.unwrap();
///
///     // Process page...
/// }
/// ```
///
/// # Note
/// This iterator consumes and mutates the `Objects` store to resolve
/// indirect references. It should not be used concurrently with other
/// operations that modify the same objects store.
#[derive(Debug)]
pub struct Pages<'a> {
    root: PagesTreeNode,
    stack: Vec<(std::vec::IntoIter<IndirectReference>, InheritableAttributes)>,
    current_iter: std::vec::IntoIter<IndirectReference>,
    current_inheritable: InheritableAttributes,
    objects: &'a mut Objects,
}

impl<'a> std::iter::Iterator for Pages<'a> {
    type Item = std::result::Result<Page, crate::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.compute_next().context(crate::error::Pages) {
            Ok(Some(val)) => Some(Ok(val)),
            Ok(None) => None,
            Err(err) => Some(Err(err.into())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(self.root.leaf_count))
    }
}

impl<'a> Pages<'a> {
    pub fn new(pages: &PagesTreeNode, objects: &'a mut Objects) -> Self {
        Self {
            root: pages.clone(),
            stack: Vec::new(),
            current_iter: pages.kids.clone().into_iter(),
            current_inheritable: pages.inheritable_attributes.clone(),
            objects,
        }
    }

    /// Computes the next page in the iteration sequence.
    ///
    /// This private method performs the actual traversal logic, following
    /// the PDF page tree structure and resolving indirect references.
    /// It uses a stack to handle nested page tree nodes and maintains
    /// inheritable attributes from parent nodes.
    ///
    /// # Returns
    /// - `Ok(Some(Page))` if a page was successfully retrieved
    /// - `Ok(None)` if there are no more pages (iteration complete)
    /// - `Err(Error)` if an error occurred during traversal or resolution
    ///
    /// # Errors
    /// Returns various errors including:
    /// - `Error::ObjectNotFound` if an indirect reference cannot be resolved
    /// - `Error::InvalidKidType` if a kid object has an unexpected type
    /// - `Error::UnexpectedNodeType` if a node type is not "Page" or "Pages"
    /// - `Error::InvalidPage` if page data cannot be parsed
    /// - `Error::InvalidPageNode` if page tree node data cannot be parsed
    fn compute_next(&mut self) -> Result<Option<Page>> {
        loop {
            if let Some(kid_ref) = self.current_iter.next() {
                let kid_obj = self
                    .objects
                    .get_object(&kid_ref)
                    .context(error::ObjectNotFound {
                        reference: kid_ref,
                        field: "Pages",
                    })?;
                let dictionary = kid_obj.as_dictionary().context(error::InvalidKidType {
                    field: "Kids",
                    indirect_reference: kid_ref,
                })?;
                let node_type = dictionary
                    .get("Type")
                    .and_then(|obj| obj.as_name().ok())
                    .context(error::FieldNotFound { field: "Type" })?;

                match node_type.as_str() {
                    "Page" => {
                        return Ok(Some(
                            Page::from_dictionary(
                                dictionary,
                                &self.current_inheritable,
                                self.objects,
                            )
                            .context(error::InvalidPage)?,
                        ));
                    }
                    "Pages" => {
                        let new_node = PagesTreeNode::from_dictionary(
                            dictionary,
                            Some(self.current_inheritable.clone()),
                        )
                        .context(error::InvalidPageNode)?;
                        let old_iter =
                            std::mem::replace(&mut self.current_iter, new_node.kids.into_iter());
                        let old_inheritable = std::mem::replace(
                            &mut self.current_inheritable,
                            new_node.inheritable_attributes,
                        );

                        self.stack.push((old_iter, old_inheritable));
                    }
                    _ => {
                        return Err(error::Error::UnexpectedNodeType {
                            got: node_type.to_string(),
                        }
                        .into());
                    }
                }
            } else if let Some((parent_iter, parent_inheritable)) = self.stack.pop() {
                self.current_iter = parent_iter;
                self.current_inheritable = parent_inheritable;
            } else {
                return Ok(None);
            }
        }
    }
}

mod error {
    use snafu::Snafu;

    use crate::types::IndirectReference;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Required field `{field}` not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display(
            "Invalid object type for field `{field}`. Used ref: `{indirect_reference}`"
        ))]
        InvalidKidType {
            field: &'static str,
            indirect_reference: IndirectReference,
            source: crate::types::object::Error,
        },

        #[snafu(display("Object with reference `{reference}` for field `{field}` not found"))]
        ObjectNotFound {
            reference: IndirectReference,
            field: &'static str,
            source: crate::objects::Error,
        },

        #[snafu(display("Unexpected node type. Got = `{got}`. Expected `Page` or `Pages`]"))]
        UnexpectedNodeType { got: String },

        #[snafu(display("Failed to read page data"))]
        InvalidPage {
            source: crate::structures::page::Error,
        },

        #[snafu(display("Failed to read page tree node data"))]
        InvalidPageNode {
            source: crate::structures::root::pages_tree::Error,
        },
    }
}

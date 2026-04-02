use crate::{HalfEdgeId, VertexId, mesh::HalfEdgeMesh};

pub struct FaceHalfedges<'a> {
    mesh: &'a HalfEdgeMesh,
    start: HalfEdgeId,
    current: Option<HalfEdgeId>,
    emitted: usize,
}

impl<'a> FaceHalfedges<'a> {
    pub(crate) fn new(mesh: &'a HalfEdgeMesh, start: HalfEdgeId) -> Self {
        Self {
            mesh,
            start,
            current: Some(start),
            emitted: 0,
        }
    }
}

impl Iterator for FaceHalfedges<'_> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.emitted += 1;
        if self.emitted > self.mesh.halfedges.len().max(1) {
            self.current = None;
            return None;
        }

        let next = self.mesh.halfedges[current.index()].next;
        self.current = if next == self.start { None } else { Some(next) };
        Some(current)
    }
}

pub struct VertexOutgoingHalfedges<'a> {
    mesh: &'a HalfEdgeMesh,
    start: HalfEdgeId,
    current: Option<HalfEdgeId>,
    emitted: usize,
}

impl<'a> VertexOutgoingHalfedges<'a> {
    pub(crate) fn new(mesh: &'a HalfEdgeMesh, start: HalfEdgeId) -> Self {
        Self {
            mesh,
            start,
            current: start.is_valid().then_some(start),
            emitted: 0,
        }
    }
}

impl Iterator for VertexOutgoingHalfedges<'_> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.emitted += 1;
        if self.emitted > self.mesh.halfedges.len().max(1) {
            self.current = None;
            return None;
        }

        let previous = self.mesh.halfedges[current.index()].prev;
        let next = self.mesh.halfedges[previous.index()].twin;
        self.current = if next == self.start { None } else { Some(next) };
        Some(current)
    }
}

pub struct FaceVertices<'a> {
    mesh: &'a HalfEdgeMesh,
    inner: FaceHalfedges<'a>,
}

impl<'a> FaceVertices<'a> {
    pub(crate) fn new(mesh: &'a HalfEdgeMesh, start: HalfEdgeId) -> Self {
        Self {
            mesh,
            inner: FaceHalfedges::new(mesh, start),
        }
    }
}

impl Iterator for FaceVertices<'_> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|halfedge| self.mesh.halfedges[halfedge.index()].origin)
    }
}

#[cfg(test)]
#[path = "iterators_tests.rs"]
mod tests;

//! Riwayat undo/redo (rencana §4.3: 100 langkah). Generik dan murni, jadi
//! mudah dites; isinya salinan model (`PadBank` + `Sequence`), yang murah
//! karena sampel dibagi lewat `Arc`.

use std::collections::VecDeque;

/// Jumlah langkah undo yang disimpan.
pub const MAX_UNDO: usize = 100;

/// Tumpukan undo dan redo.
#[derive(Debug, Clone)]
pub struct History<T> {
    undo: VecDeque<T>,
    redo: Vec<T>,
    batas: usize,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self::new(MAX_UNDO)
    }
}

impl<T> History<T> {
    /// Riwayat dengan paling banyak `batas` langkah undo.
    #[must_use]
    pub fn new(batas: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            batas: batas.max(1),
        }
    }

    /// Simpan keadaan **sebelum** sebuah perubahan. Redo dikosongkan, karena
    /// cabang lama tidak berlaku lagi. Langkah tertua dibuang bila penuh.
    pub fn checkpoint(&mut self, sebelum: T) {
        if self.undo.len() == self.batas {
            self.undo.pop_front();
        }
        self.undo.push_back(sebelum);
        self.redo.clear();
    }

    /// Mundur satu langkah: `sekarang` masuk redo, keadaan sebelumnya
    /// dikembalikan untuk diterapkan. `None` bila tidak ada.
    pub fn undo(&mut self, sekarang: T) -> Option<T> {
        let lama = self.undo.pop_back()?;
        self.redo.push(sekarang);
        Some(lama)
    }

    /// Maju satu langkah (kebalikan undo).
    pub fn redo(&mut self, sekarang: T) -> Option<T> {
        let baru = self.redo.pop()?;
        self.undo.push_back(sekarang);
        Some(baru)
    }

    /// (jumlah undo, jumlah redo) yang tersedia.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn undo_lalu_redo() {
        let mut r = History::new(10);
        r.checkpoint(1); // sebelum mengubah 1 → 2
        r.checkpoint(2); // sebelum mengubah 2 → 3
        assert_eq!(r.undo(3), Some(2));
        assert_eq!(r.undo(2), Some(1));
        assert_eq!(r.undo(1), None);
        assert_eq!(r.redo(1), Some(2));
        assert_eq!(r.redo(2), Some(3));
        assert_eq!(r.redo(3), None);
    }

    #[test]
    fn perubahan_baru_menghapus_redo() {
        let mut r = History::new(10);
        r.checkpoint(1);
        assert_eq!(r.undo(2), Some(1));
        assert_eq!(r.counts(), (0, 1));
        r.checkpoint(1); // mengubah 1 → 5
        assert_eq!(r.counts(), (1, 0));
        assert_eq!(r.redo(5), None);
    }

    #[test]
    fn hanya_seratus_langkah() {
        let mut r = History::default();
        for i in 0..150 {
            r.checkpoint(i);
        }
        assert_eq!(r.counts().0, 100);
        let mut terakhir = 150;
        while let Some(v) = r.undo(terakhir) {
            terakhir = v;
        }
        assert_eq!(terakhir, 50, "langkah tertua dibuang");
    }
}

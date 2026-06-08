;;; ax_plotting.lisp — Lisp helpers for Aximar plotting functions
;;;
;;; Loaded during session init before ax_plotting.mac.
;;; Defines functions callable from Maxima code.

(in-package :maxima)

;; Counter for generating unique temp file names.
(defvar *ax--plot-counter* 0)

;; Isolated random state so we don't disturb the user's *random-state*.
;; (make-random-state t) uses OS entropy — each process gets a different seed.
(defvar *ax--plot-random-state* (make-random-state t))

;; Create a unique temp file path with .plotly.json extension.
;; Counter ensures within-process uniqueness; isolated RNG ensures
;; cross-process uniqueness.  Fully portable — no sb-posix dependency.
;; Returns the path as a Maxima string.
(defun $ax__mktemp ()
  (incf *ax--plot-counter*)
  (format nil "~A/ax_plot_~9,'0D_~D.plotly.json"
    $maxima_tempdir
    (random 1000000000 *ax--plot-random-state*)
    *ax--plot-counter*))

;;; Optional ndarray support (numerics package).
;;; All symbol resolution is deferred to call time so ax-plots compiles
;;; and loads without numerics present. Load order does not matter.

(defun $ax__ndarray_p (x)
  "Test if X is an ndarray from the numerics package.
   Returns T if the numerics package is loaded and X has the
   (($ndarray simp) <struct>) form."
  (and (find-package :numerics)
       (listp x)
       (listp (car x))
       (eq (caar x) '$ndarray)
       (cdr x)
       t))

(defun ax--resolve (pkg-name sym-name)
  "Resolve a symbol by name at runtime. Returns nil if not found."
  (let ((pkg (find-package pkg-name)))
    (when pkg (find-symbol sym-name pkg))))

(defun $ax__ndarray_to_list (x)
  "Convert an ndarray to a Maxima list suitable for plotting.
   1D (n,)     -> flat mlist
   2D (n,2)    -> list of [x,y] pairs
   2D (n,3)    -> list of [x,y,z] triples
   2D (n,m)    -> list of row-lists"
  (let* ((handle (cadr x))
         (tensor-fn (ax--resolve :numerics "NDARRAY-TENSOR"))
         (tensor (funcall tensor-fn handle))
         (shape-fn (ax--resolve :magicl "SHAPE"))
         (tref-fn (ax--resolve :magicl "TREF"))
         (shape (funcall shape-fn tensor)))
    (if (= (length shape) 1)
        ;; 1D: flat list
        (let ((n (first shape)))
          `((mlist simp) ,@(loop for i below n
                                 collect (funcall tref-fn tensor i))))
        ;; 2D: list of row-lists
        (let ((nrow (first shape))
              (ncol (second shape)))
          `((mlist simp)
            ,@(loop for i below nrow
                    collect `((mlist simp)
                              ,@(loop for j below ncol
                                      collect (funcall tref-fn tensor i j)))))))))

(defun $ax__ndarray_to_matrix (x)
  "Convert an ndarray to a Maxima matrix via np_to_matrix.
   Falls back to $ax__ndarray_to_list if np_to_matrix is unavailable."
  (if (fboundp '$np_to_matrix)
      (funcall (symbol-function '$np_to_matrix) x)
      ;; Fallback: build matrix from list-of-rows
      (let ((lst ($ax__ndarray_to_list x)))
        `(($matrix simp) ,@(cdr lst)))))

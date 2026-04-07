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

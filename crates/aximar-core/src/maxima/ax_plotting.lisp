;;; ax_plotting.lisp — Lisp helpers for Aximar plotting functions
;;;
;;; Loaded during session init before ax_plotting.mac.
;;; Defines functions callable from Maxima code.

(in-package :maxima)

;; Create a unique temp file with .plotly.json extension using mkstemp.
;; Returns the path as a Maxima string.
(defun $ax__mktemp ()
  (multiple-value-bind (fd path)
      (sb-posix:mkstemp
       (format nil "~A/ax_plot_XXXXXX" $maxima_tempdir))
    (sb-posix:close fd)
    (let ((final (concatenate 'string path ".plotly.json")))
      (rename-file path final)
      final)))

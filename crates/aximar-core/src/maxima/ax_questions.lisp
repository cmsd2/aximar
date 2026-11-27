;; Maxima's questions become errors.
;;
;; Maxima asks every question ("Is a positive, negative or zero?", "Is n an
;; integer?") through RETRIEVE, which prints it and reads an answer from
;; the input.  Aximar sends a cell's input in one block, so an answer it
;; can't give hangs the cell or leaks into the next one.  Instead the
;; question becomes an ordinary Maxima error, "Maxima needs to know:
;; <question>", which unwinds like any other: errcatch catches it, and
;; the cell ends with an error output.  Aximar explains it from the
;; question's English (maxima/errors.rs).
;;
;; Loaded as one :lisp line when each Maxima process starts.  Comments
;; must take whole lines; see plotting::lisp_stdin_line.

;; The question as plain text, as RETRIEVE would print it
(defun ax-question-text (msg flag)
  (cond ((null msg) "")
        ((stringp msg) msg)
        ((atom msg) (princ-to-string msg))
        (t (let ((parts (cond ((eq flag t) (cdr msg))
                              ((and (consp (car msg)) (eq (caar msg) 'mtext)) (cdr msg))
                              (t (list msg)))))
             (apply #'concatenate 'string
                    (mapcar #'(lambda (p) (if (stringp p) p (coerce (mstring p) 'string)))
                            parts))))))

;; The whole question goes in MERROR's pattern, with no arguments, because
;; kernel-events reports an error's pattern without them.  A ~ in the
;; question is doubled so FORMAT prints it as it is.
(defun ax-format-escape (text)
  (with-output-to-string (out)
    (loop for c across text
          do (when (char= c #\~) (write-char #\~ out))
             (write-char c out))))

(when (fboundp 'retrieve)
  (setf (symbol-function 'retrieve)
        #'(lambda (msg flag)
            (merror (concatenate 'string "Maxima needs to know: "
                                 (ax-format-escape (ax-question-text msg flag)))))))

;; Nothing to print
(values)

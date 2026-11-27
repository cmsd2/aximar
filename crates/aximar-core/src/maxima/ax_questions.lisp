;; Maxima's questions stop the statement with an error.
;;
;; Maxima asks every question ("Is a positive, negative or zero?", "Is n an
;; integer?") through RETRIEVE, which prints it and reads an answer from
;; the input.  Aximar sends a cell's input in one block, so an answer it
;; can't give hangs the cell or leaks into the next one.  Instead a
;; question stops the top-level statement with an ordinary Maxima error,
;; "Maxima needs to know: <question>", and the cell ends with an error
;; output that Aximar explains from the question's English
;; (maxima/errors.rs).
;;
;; A question isn't an error in Maxima: errcatch doesn't stop it reaching
;; the user, and library code relies on that.  laplace, for one, runs
;; defint inside errset and falls back to another method on an error, so a
;; question raised as an error there would be swallowed and the result
;; would change without a word.  So RETRIEVE throws the question to a
;; catch around each top-level statement, past every errcatch and errset
;; (their cleanup still runs), and the error is raised there.
;;
;; Loaded as one :lisp line when each Maxima process starts, before
;; kernel-events, so its wrapper sees the error.  Comments must take whole
;; lines; see plotting::lisp_stdin_line.

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

;; The error, raised as if nothing caught it, and printed even with
;; errormsg false: the question must reach the user
(defun ax-question-error (question)
  (let ((errcatch nil) (*merror-signals-$error-p* nil) ($errormsg t))
    (merror (concatenate 'string "Maxima needs to know: " (ax-format-escape question)))))

;; True inside a top-level statement, where the catch for questions is
(defvar *ax-question-catcher* nil)
(defvar *ax-original-toplevel-eval* nil)
(defvar *ax-original-dbm-loop* nil)

(when (and (fboundp 'retrieve)
           (fboundp 'toplevel-macsyma-eval)
           (null *ax-original-toplevel-eval*))
  (setq *ax-original-toplevel-eval* (symbol-function 'toplevel-macsyma-eval))
  (setf (symbol-function 'toplevel-macsyma-eval)
        #'(lambda (x)
            (let* ((asked t)
                   (result nil)
                   (question (catch 'ax-question
                               (let ((*ax-question-catcher* t))
                                 (setq result (funcall *ax-original-toplevel-eval* x))
                                 (setq asked nil)))))
              (if asked (ax-question-error question) result))))
  ;; In the debugger a question stays where it is asked, without
  ;; unwinding out of the stopped frame
  (when (fboundp 'break-dbm-loop)
    (setq *ax-original-dbm-loop* (symbol-function 'break-dbm-loop))
    (setf (symbol-function 'break-dbm-loop)
          #'(lambda (&rest args)
              (let ((*ax-question-catcher* nil))
                (apply *ax-original-dbm-loop* args)))))
  (setf (symbol-function 'retrieve)
        #'(lambda (msg flag)
            (let ((question (ax-question-text msg flag)))
              (if *ax-question-catcher*
                  (throw 'ax-question question)
                  (ax-question-error question))))))

;; Nothing to print
(values)

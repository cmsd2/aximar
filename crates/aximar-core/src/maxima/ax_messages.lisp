;;; ax_messages.lisp: Maxima's error messages in the user's language
;;;
;;; Aximar starts Maxima in English (LANGUAGE=en) and reads its messages
;;; to tell when a cell failed.  To show the user an error in their own
;;; language, this wraps errormsg(), which prints every error that merror
;;; raises: at the top level, inside errcatch and in the debugger.  After
;;; the English message, the wrapper looks the message's pattern up in
;;; Maxima's own catalog for ax__language, formats it with the error's
;;; arguments, and prints it as one tagged line, newlines escaped:
;;;
;;;   __AXIMAR_MSG__diff: numero de argumentos equivocados.__AXIMAR_MSG_END__
;;;
;;; It prints nothing when ax__language is empty or the catalog has no
;;; translation, so the English message stands alone.  The tagged line
;;; comes out at error time, before any prompt, so it needs no query
;;; afterwards and works in the debugger too.
;;;
;;; Loaded during session init, sent as one :lisp line, so this file has
;;; no semicolons except in comments.

(defvar $ax__language "")

(defvar *ax-original-errormsg* nil)

(defun ax-escape-message (text)
  (with-output-to-string (out)
    (loop for ch across text
          do (cond ((char= ch #\Newline) (write-string "\\n" out))
                   ((char= ch #\\) (write-string "\\\\" out))
                   (t (write-char ch out))))))

(defun ax-translated-error-message ()
  (let ((lang $ax__language)
        (pattern (cadr $error)))
    (when (and (stringp lang) (plusp (length lang)) (stringp pattern))
      (let ((translated (let ((intl::*locale* lang))
                          (intl:dgettext "maxima" pattern))))
        (when (and (stringp translated) (string/= translated pattern))
          (string-trim '(#\Newline #\Space)
                       (with-output-to-string (*standard-output*)
                         (let (($error (list* (car $error) translated (cddr $error))))
                           (funcall *ax-original-errormsg*)))))))))

(unless *ax-original-errormsg*
  (setf *ax-original-errormsg* (symbol-function '$errormsg))
  (setf (symbol-function '$errormsg)
        (lambda ()
          (prog1 (funcall *ax-original-errormsg*)
            (let ((text (ignore-errors (ax-translated-error-message))))
              (when (and text (plusp (length text)))
                (format t "__AXIMAR_MSG__~a__AXIMAR_MSG_END__~%"
                        (ax-escape-message text)))))))
  t)

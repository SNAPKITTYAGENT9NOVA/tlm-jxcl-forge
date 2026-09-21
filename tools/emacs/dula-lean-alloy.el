;;; dula-lean-alloy.el --- Lean/Alloy counterlemma and assertion library -*- lexical-binding: t; -*-

;; DULA: Deterministic Universal Lemma Analysis
;; A standalone Emacs Lisp library.  No Org-Babel dependency.
;;
;; Design:
;;   Lean source -> assertion point -> semantic proposition -> Alloy model
;;   -> bounded counterexample -> counterlemma -> recursive Lean assertion.
;;
;; Lean remains the proof authority. Alloy is a bounded countermodel engine.
;; The library deliberately labels Alloy results as bounded evidence.

(require 'cl-lib)
(require 'seq)
(require 'subr-x)
(require 'json)

(defgroup dula-lean-alloy nil
  "Lean/Alloy counterlemma and recursive assertion tooling."
  :group 'languages)

(defcustom dula-lean-command "lean"
  "Lean executable."
  :type 'string :group 'dula-lean-alloy)

(defcustom dula-alloy-command "java"
  "Java executable used to launch Alloy when configured."
  :type 'string :group 'dula-lean-alloy)

(defcustom dula-alloy-jar nil
  "Path to an Alloy Analyzer jar.
The library also supports an external Alloy command through
`dula-alloy-run-command' when a jar is not configured."
  :type '(choice (const :tag "Auto/none" nil) file)
  :group 'dula-lean-alloy)

(defcustom dula-alloy-run-command nil
  "External command template for Alloy.
The string receives the temporary Alloy file path as its final argument."
  :type '(choice (const nil) string)
  :group 'dula-lean-alloy)

(defcustom dula-default-scope 5
  "Default Alloy scope for counterlemma searches."
  :type 'integer :group 'dula-lean-alloy)

(defcustom dula-counterexample-directory
  (expand-file-name "dula-counterexamples/" temporary-file-directory)
  "Directory for retained Alloy and Lean artifacts."
  :type 'directory :group 'dula-lean-alloy)

(defcustom dula-recursion-limit 32
  "Maximum recursive assertion depth."
  :type 'integer :group 'dula-lean-alloy)

(defcustom dula-functor-strict t
  "When non-nil, functor bindings must declare source and target sorts."
  :type 'boolean :group 'dula-lean-alloy)

(defconst dula-version "0.1.0")
(defconst dula-schema-version 1)

(cl-defstruct dula-proposition
  id name kind type source assumptions conclusion metadata)

(cl-defstruct dula-functor
  id name source target arity variance body metadata)

(cl-defstruct dula-assertion
  id name proposition assumptions source line column status
  alloy-file lean-file counterexample parent depth metadata)

(cl-defstruct dula-counterlemma
  id assertion scope model formula witness status lean-statement
  alloy-file created-at metadata)

(cl-defstruct dula-node
  id kind payload parents children depth metadata)

(defvar dula--propositions (make-hash-table :test #'equal))
(defvar dula--functors (make-hash-table :test #'equal))
(defvar dula--assertions (make-hash-table :test #'equal))
(defvar dula--counterlemmas (make-hash-table :test #'equal))
(defvar dula--nodes (make-hash-table :test #'equal))
(defvar dula--assertion-stack nil)
(defvar dula--current-source nil)
(defvar dula--last-result nil)

(defun dula--uuid (prefix)
  "Create a process-local stable identifier with PREFIX."
  (format "%s-%x-%x" prefix (float-time) (random most-positive-fixnum)))

(defun dula--string (value)
  "Convert VALUE to a non-nil string."
  (if (stringp value) value (format "%s" value)))

(defun dula--nonempty (value)
  "Return VALUE when it is a non-empty string."
  (and value (stringp value) (not (string-empty-p (string-trim value))) value))

(defun dula--safe-symbol (name)
  "Turn NAME into a Lisp symbol without evaluating it."
  (intern (replace-regexp-in-string "[^[:alnum:]_-]" "_" (dula--string name))))

(defun dula--ensure-directory ()
  "Create the artifact directory when necessary."
  (make-directory dula-counterexample-directory t)
  dula-counterexample-directory)

(defun dula--write-file (path text)
  "Write TEXT to PATH using UTF-8."
  (make-directory (file-name-directory path) t)
  (with-temp-file path
    (set-buffer-file-coding-system 'utf-8-unix)
    (insert text))
  path)

(defun dula--read-file (path)
  "Read PATH as UTF-8."
  (with-temp-buffer
    (insert-file-contents path)
    (buffer-string)))

(defun dula--shell-output (program args)
  "Run PROGRAM with ARGS and return `(STATUS . OUTPUT)'."
  (with-temp-buffer
    (let ((status (apply #'process-file program nil '(t t) nil args)))
      (cons status (buffer-string)))))

(defun dula--timestamp ()
  "Return an ISO-like timestamp."
  (format-time-string "%Y-%m-%dT%H:%M:%S%z"))

(defun dula--plist-get* (plist key default)
  "Read KEY from PLIST, returning DEFAULT when absent."
  (if (plist-member plist key) (plist-get plist key) default))

(defun dula--hash-values (table)
  "Return all values in TABLE."
  (let (values)
    (maphash (lambda (_ value) (push value values)) table)
    (nreverse values)))

(defun dula--hash-keys (table)
  "Return all keys in TABLE."
  (let (keys)
    (maphash (lambda (key _value) (push key keys)) table)
    (nreverse keys)))

(defun dula-reset ()
  "Reset all in-memory DULA state."
  (interactive)
  (clrhash dula--propositions)
  (clrhash dula--functors)
  (clrhash dula--assertions)
  (clrhash dula--counterlemmas)
  (clrhash dula--nodes)
  (setq dula--assertion-stack nil
        dula--current-source nil
        dula--last-result nil)
  t)

;;; Semantic propositions

(defun dula-proposition-create (name kind &optional type source metadata)
  "Create and register semantic proposition NAME of KIND."
  (let ((id (dula--uuid "prop")))
    (let ((p (make-dula-proposition :id id :name name :kind kind
                                    :type type :source source
                                    :metadata metadata)))
      (puthash id p dula--propositions)
      p)))

(defun dula-proposition-get (id)
  "Return proposition ID."
  (gethash id dula--propositions))

(defun dula-proposition-by-name (name)
  "Return the first proposition named NAME."
  (seq-find (lambda (p) (equal name (dula-proposition-name p)))
            (dula--hash-values dula--propositions)))

(defun dula-proposition-atom (name &optional type source)
  "Create an atomic semantic proposition."
  (dula-proposition-create name 'atom type source))

(defun dula-proposition-not (p)
  "Create semantic negation of proposition P."
  (dula-proposition-create (format "not(%s)" (dula-proposition-name p))
                           'not nil nil (list :operand (dula-proposition-id p))))

(defun dula-proposition-and (&rest propositions)
  "Create a conjunction from PROPOSITIONS."
  (dula-proposition-create
   (format "and(%s)" (mapconcat #'dula-proposition-name propositions ","))
   'and nil nil (list :operands (mapcar #'dula-proposition-id propositions))))

(defun dula-proposition-or (&rest propositions)
  "Create a disjunction from PROPOSITIONS."
  (dula-proposition-create
   (format "or(%s)" (mapconcat #'dula-proposition-name propositions ","))
   'or nil nil (list :operands (mapcar #'dula-proposition-id propositions))))

(defun dula-proposition-implies (antecedent consequent)
  "Create semantic implication."
  (dula-proposition-create
   (format "(%s)->(%s)" (dula-proposition-name antecedent)
           (dula-proposition-name consequent))
   'implies nil nil
   (list :antecedent (dula-proposition-id antecedent)
         :consequent (dula-proposition-id consequent))))

(defun dula-proposition-equivalent (left right)
  "Create semantic equivalence."
  (dula-proposition-create
   (format "(%s)<->(%s)" (dula-proposition-name left)
           (dula-proposition-name right))
   'equivalent nil nil
   (list :left (dula-proposition-id left) :right (dula-proposition-id right))))

(defun dula-proposition-assumption (name &optional type source)
  "Create an assumption proposition."
  (dula-proposition-create name 'assumption type source))

(defun dula-proposition-conclusion (name &optional type source)
  "Create a conclusion proposition."
  (dula-proposition-create name 'conclusion type source))

(defun dula-proposition-expression (p)
  "Return a recursively rendered semantic expression for P."
  (pcase (dula-proposition-kind p)
    ('atom (dula-proposition-name p))
    ('assumption (dula-proposition-name p))
    ('conclusion (dula-proposition-name p))
    ('not (format "not (%s)" (dula-proposition-expression
                                (dula-proposition-get
                                 (plist-get (dula-proposition-metadata p) :operand)))))
    ('and (format "and (%s)"
                  (mapconcat
                   (lambda (id) (dula-proposition-expression (dula-proposition-get id)))
                   (plist-get (dula-proposition-metadata p) :operands) " ")))
    ('or (format "or (%s)"
                 (mapconcat
                  (lambda (id) (dula-proposition-expression (dula-proposition-get id)))
                  (plist-get (dula-proposition-metadata p) :operands) " ")))
    ('implies (format "(%s) -> (%s)"
                      (dula-proposition-expression
                       (dula-proposition-get
                        (plist-get (dula-proposition-metadata p) :antecedent)))
                      (dula-proposition-expression
                       (dula-proposition-get
                        (plist-get (dula-proposition-metadata p) :consequent)))))
    ('equivalent (format "(%s) <-> (%s)"
                         (dula-proposition-expression
                          (dula-proposition-get
                           (plist-get (dula-proposition-metadata p) :left)))
                         (dula-proposition-expression
                          (dula-proposition-get
                           (plist-get (dula-proposition-metadata p) :right)))))
    (_ (dula-proposition-name p))))

(defun dula-proposition-to-alloy (p)
  "Translate semantic proposition P to an Alloy boolean expression."
  (pcase (dula-proposition-kind p)
    ((or 'atom 'assumption 'conclusion) (dula-alloy-ident (dula-proposition-name p)))
    ('not (format "not (%s)"
                  (dula-proposition-to-alloy
                   (dula-proposition-get
                    (plist-get (dula-proposition-metadata p) :operand)))))
    ('and (format "(%s)"
                  (mapconcat
                   (lambda (id) (dula-proposition-to-alloy (dula-proposition-get id)))
                   (plist-get (dula-proposition-metadata p) :operands) " and ")))
    ('or (format "(%s)"
                 (mapconcat
                  (lambda (id) (dula-proposition-to-alloy (dula-proposition-get id)))
                  (plist-get (dula-proposition-metadata p) :operands) " or ")))
    ('implies (format "(%s) implies (%s)"
                      (dula-proposition-to-alloy
                       (dula-proposition-get
                        (plist-get (dula-proposition-metadata p) :antecedent)))
                      (dula-proposition-to-alloy
                       (dula-proposition-get
                        (plist-get (dula-proposition-metadata p) :consequent)))))
    ('equivalent (format "(%s) iff (%s)"
                         (dula-proposition-to-alloy
                          (dula-proposition-get
                           (plist-get (dula-proposition-metadata p) :left)))
                         (dula-proposition-to-alloy
                          (dula-proposition-get
                           (plist-get (dula-proposition-metadata p) :right)))))
    (_ "univ = univ")))

(defun dula-alloy-ident (name)
  "Make NAME a conservative Alloy identifier."
  (let ((s (replace-regexp-in-string "[^A-Za-z0-9_]" "_" (dula--string name))))
    (if (string-match-p "\\`[0-9]" s) (concat "p_" s) s)))

;;; Functor bindings

(defun dula-functor-bind (name source target &optional body metadata)
  "Bind functor NAME from SOURCE to TARGET.
BODY is an optional semantic expression or implementation description."
  (when (and dula-functor-strict (or (null source) (null target)))
    (user-error "Functor %s requires source and target" name))
  (let ((f (make-dula-functor :id (dula--uuid "fun")
                               :name name :source source :target target
                               :arity 1 :variance 'covariant :body body
                               :metadata metadata)))
    (puthash (dula-functor-id f) f dula--functors)
    f))

(defun dula-functor-get (id)
  "Return functor ID."
  (gethash id dula--functors))

(defun dula-functor-by-name (name)
  "Return functor named NAME."
  (seq-find (lambda (f) (equal name (dula-functor-name f)))
            (dula--hash-values dula--functors)))

(defun dula-functor-map (functor proposition)
  "Bind FUNCTOR to PROPOSITION as a mapped proposition."
  (dula-proposition-create
   (format "%s(%s)" (dula-functor-name functor)
           (dula-proposition-name proposition))
   'atom
   (dula-functor-target functor)
   (dula-proposition-source proposition)
   (list :functor (dula-functor-id functor)
         :argument (dula-proposition-id proposition))))

(defun dula-functor-compose (outer inner)
  "Return a composed functor binding OUTER after INNER."
  (unless (equal (dula-functor-source outer) (dula-functor-target inner))
    (user-error "Functor types do not compose: %s after %s"
                (dula-functor-name outer) (dula-functor-name inner)))
  (dula-functor-bind
   (format "%s∘%s" (dula-functor-name outer) (dula-functor-name inner))
   (dula-functor-source inner)
   (dula-functor-target outer)
   (list :compose (dula-functor-id outer) (dula-functor-id inner))))

(defun dula-functor-law-identity (functor)
  "Create an identity-law assertion for FUNCTOR."
  (dula-proposition-atom
   (format "id_%s_law" (dula-functor-name functor))))

(defun dula-functor-law-composition (outer inner)
  "Create a composition-law proposition for two functors."
  (dula-proposition-atom
   (format "%s_%s_composition_law"
           (dula-functor-name outer) (dula-functor-name inner))))

;;; Assertion points

(defun dula-assertion-create (name proposition &optional assumptions source line column metadata)
  "Create an assertion point for PROPOSITION."
  (let ((a (make-dula-assertion
            :id (dula--uuid "assert") :name name :proposition proposition
            :assumptions assumptions :source source :line line :column column
            :status 'pending :metadata metadata :depth (length dula--assertion-stack))))
    (puthash (dula-assertion-id a) a dula--assertions)
    a))

(defun dula-assertion-get (id)
  "Return assertion ID."
  (gethash id dula--assertions))

(defun dula-assertion-by-name (name)
  "Return assertion named NAME."
  (seq-find (lambda (a) (equal name (dula-assertion-name a)))
            (dula--hash-values dula--assertions)))

(defun dula-assertion-push (assertion)
  "Push ASSERTION onto the recursive assertion stack."
  (push assertion dula--assertion-stack)
  assertion)

(defun dula-assertion-pop ()
  "Pop the current recursive assertion point."
  (pop dula--assertion-stack))

(defun dula-current-assertion ()
  "Return the current recursive assertion point."
  (car dula--assertion-stack))

(defun dula-assertion-formula (assertion)
  "Construct implication formula for ASSERTION."
  (let ((conclusion (dula-assertion-proposition assertion))
        (assumptions (dula-assertion-assumptions assertion)))
    (if assumptions
        (dula-proposition-implies
         (apply #'dula-proposition-and assumptions)
         conclusion)
      conclusion)))

(defun dula-assertion-counterformula (assertion)
  "Construct the Alloy counterformula for ASSERTION."
  (let ((formula (dula-assertion-formula assertion)))
    (dula-proposition-not formula)))

;;; Alloy generation

(defun dula-alloy-header ()
  "Return the DULA Alloy model header."
  (concat "module dula_counterlemma\n\n"
          "sig State {}\n"
          "sig Proposition {}\n"
          "sig Lemma { assumptions: set Proposition, conclusion: one Proposition }\n\n"))

(defun dula-alloy-proposition-declarations (assertion)
  "Return proposition declarations needed by ASSERTION."
  (let ((props (append (dula-assertion-assumptions assertion)
                       (list (dula-assertion-proposition assertion)))))
    (mapconcat
     (lambda (p)
       (format "one sig %s extends Proposition {}"
               (dula-alloy-ident (dula-proposition-name p))))
     (delete-dups props) "\n")))

(defun dula-alloy-assumption-expression (assertion)
  "Return conjunction of ASSERTION assumptions."
  (if (dula-assertion-assumptions assertion)
      (mapconcat #'dula-proposition-to-alloy
                 (dula-assertion-assumptions assertion) " and ")
    "univ = univ"))

(defun dula-alloy-conclusion-expression (assertion)
  "Return Alloy expression for ASSERTION conclusion."
  (dula-proposition-to-alloy (dula-assertion-proposition assertion)))

(defun dula-alloy-counterformula (assertion)
  "Return a direct Alloy countermodel predicate for ASSERTION."
  (format "pred Countermodel[s: State] { (%s) and not (%s) }"
          (dula-alloy-assumption-expression assertion)
          (dula-alloy-conclusion-expression assertion)))

(defun dula-alloy-check (assertion scope)
  "Return Alloy source checking ASSERTION at SCOPE."
  (concat
   (dula-alloy-header)
   (dula-alloy-proposition-declarations assertion) "\n\n"
   (dula-alloy-counterformula assertion) "\n\n"
   "assert NoCountermodel { all s: State | not Countermodel[s] }\n"
   (format "check NoCountermodel for %d\n" scope)
   (format "run { some s: State | Countermodel[s] } for %d\n" scope)))

(defun dula-alloy-write (assertion scope)
  "Write an Alloy model for ASSERTION at SCOPE and return its path."
  (dula--ensure-directory)
  (let ((path (expand-file-name
               (format "%s.als" (dula-assertion-id assertion))
               dula-counterexample-directory)))
    (dula--write-file path (dula-alloy-check assertion scope))
    (setf (dula-assertion-alloy-file assertion) path)
    path))

;;; Alloy execution adapters

(defun dula-alloy-command-line (file)
  "Build a command line for Alloy FILE."
  (cond
   (dula-alloy-run-command
    (append (split-string-and-unquote dula-alloy-run-command) (list file)))
   (dula-alloy-jar
    (list dula-alloy-command "-jar" dula-alloy-jar file))
   (t nil)))

(defun dula-alloy-run (file)
  "Run Alloy adapter against FILE.
Returns `(STATUS . OUTPUT)'."
  (let ((command (dula-alloy-command-line file)))
    (unless command
      (user-error "No Alloy runner configured; set dula-alloy-jar or dula-alloy-run-command"))
    (dula--shell-output (car command) (cdr command))))

(defun dula-alloy-output-counterexample-p (output)
  "Heuristically detect an Alloy counterexample in OUTPUT."
  (or (string-match-p "Counterexample" output)
      (string-match-p "SAT" output)
      (string-match-p "Instance" output)
      (string-match-p "found" output)))

(defun dula-alloy-output-unsat-p (output)
  "Heuristically detect an Alloy UNSAT result in OUTPUT."
  (or (string-match-p "UNSAT" output)
      (string-match-p "No instance" output)
      (string-match-p "no counterexample" (downcase output))))

;;; Counterlemma creation

(defun dula-counterlemma-create (assertion scope output)
  "Create a counterlemma artifact from ASSERTION, SCOPE and Alloy OUTPUT."
  (let* ((formula (dula-assertion-counterformula assertion))
         (found (dula-alloy-output-counterexample-p output))
         (status (if found 'counterexample-found 'no-counterexample-found))
         (c (make-dula-counterlemma
             :id (dula--uuid "counter")
             :assertion (dula-assertion-id assertion)
             :scope scope
             :model output
             :formula formula
             :witness output
             :status status
             :lean-statement (dula-counterlemma-lean-statement assertion)
             :alloy-file (dula-assertion-alloy-file assertion)
             :created-at (dula--timestamp))))
    (puthash (dula-counterlemma-id c) c dula--counterlemmas)
    (setf (dula-assertion-counterexample assertion) c
          (dula-assertion-status assertion) status)
    c))

(defun dula-counterlemma-lean-statement (assertion)
  "Produce a Lean-facing statement for ASSERTION's counterlemma."
  (let ((name (dula-assertion-name assertion)))
    (format "-- DULA counterlemma for %s\n-- Alloy result is bounded evidence.\nexample : False := by\n  fail_if_success exact by contradiction\n  sorry" name)))

(defun dula-counterlemma-get (id)
  "Return counterlemma ID."
  (gethash id dula--counterlemmas))

(defun dula-counterlemma-write-lean (counterlemma)
  "Write a Lean witness target for COUNTERLEMMA."
  (dula--ensure-directory)
  (let ((path (expand-file-name
               (format "%s.lean" (dula-counterlemma-id counterlemma))
               dula-counterexample-directory)))
    (dula--write-file path (dula-counterlemma-lean-statement
                            (dula-assertion-get
                             (dula-counterlemma-assertion counterlemma))))
    path))

;;; Recursive assertion engine

(defun dula-recursive-assert (assertion &optional scope depth)
  "Recursively analyze ASSERTION with Alloy and its parent assertions."
  (setq scope (or scope dula-default-scope)
        depth (or depth 0))
  (when (> depth dula-recursion-limit)
    (user-error "DULA recursion limit exceeded at depth %d" depth))
  (setf (dula-assertion-depth assertion) depth)
  (dula-assertion-push assertion)
  (unwind-protect
      (let ((parents (plist-get (dula-assertion-metadata assertion) :parents)))
        (dolist (parent-id parents)
          (let ((parent (dula-assertion-get parent-id)))
            (when parent
              (dula-recursive-assert parent scope (1+ depth)))))
        (let* ((file (dula-alloy-write assertion scope))
               (result (condition-case err
                           (dula-alloy-run file)
                         (error (cons 255 (error-message-string err)))))
               (counter (dula-counterlemma-create assertion scope (cdr result))))
          (setq dula--last-result counter)
          counter))
    (dula-assertion-pop)))

(defun dula-recursive-chain (assertions &optional scope)
  "Analyze ASSERTIONS in order, linking each result to the previous one."
  (let ((previous nil) results)
    (dolist (assertion assertions (nreverse results))
      (when previous
        (setf (dula-assertion-metadata assertion)
              (plist-put (dula-assertion-metadata assertion)
                         :parents (list (dula-assertion-id previous)))))
      (push (dula-recursive-assert assertion scope) results)
      (setq previous assertion))))

;;; Lean assertion points

(defun dula-lean-find-assertion-points (source)
  "Find lightweight assertion points in Lean SOURCE.
Recognizes `theorem', `lemma', `example', and `assert' declarations."
  (let (points)
    (with-temp-buffer
      (insert source)
      (goto-char (point-min))
      (while (re-search-forward
              "^[[:space:]]*\\(theorem\\|lemma\\|example\\|assert\\)[[:space:]]+\\([A-Za-z0-9_']+\\)" nil t)
        (push (list :kind (match-string 1)
                    :name (match-string 2)
                    :line (line-number-at-pos)
                    :column (current-column))
              points)))
    (nreverse points)))

(defun dula-lean-extract-proposition-text (source name)
  "Extract a conservative proposition fragment associated with NAME."
  (with-temp-buffer
    (insert source)
    (goto-char (point-min))
    (when (re-search-forward
           (format "^[[:space:]]*\\(?:theorem\\|lemma\\|example\\|assert\\)[[:space:]]+%s\\b" (regexp-quote name))
           nil t)
      (let ((start (line-beginning-position)))
        (if (re-search-forward "^[[:space:]]*\\(?:by\\|:=\\)" nil t)
            (string-trim (buffer-substring-no-properties start (line-beginning-position)))
          (string-trim (buffer-substring-no-properties start (line-end-position))))))))

(defun dula-lean-assertion-from-point (source point)
  "Create a DULA assertion from POINT metadata and SOURCE."
  (let* ((name (plist-get point :name))
         (text (dula-lean-extract-proposition-text source name))
         (prop (dula-proposition-atom text 'LeanProp source)))
    (dula-assertion-create name prop nil source
                           (plist-get point :line)
                           (plist-get point :column)
                           (list :lean-kind (plist-get point :kind)
                                 :lean-text text))))

(defun dula-lean-register-source (source)
  "Register assertion points found in Lean SOURCE."
  (setq dula--current-source source)
  (mapcar (lambda (point) (dula-lean-assertion-from-point source point))
          (dula-lean-find-assertion-points source)))

;;; Recursive Lean compilation

(defun dula-lean-run-file (file)
  "Compile Lean FILE and return `(STATUS . OUTPUT)'."
  (dula--shell-output dula-lean-command (list "--json" file)))

(defun dula-lean-run-source (source)
  "Compile SOURCE in a temporary Lean file."
  (let ((file (make-temp-file "dula-lean-" nil ".lean")))
    (dula--write-file file source)
    (dula-lean-run-file file)))

(defun dula-lean-success-p (result)
  "Return non-nil when Lean RESULT has exit status zero."
  (zerop (car result)))

(defun dula-lean-errors (result)
  "Extract error-looking lines from Lean RESULT."
  (seq-filter (lambda (line) (string-match-p "error" (downcase line)))
              (split-string (cdr result) "\n" t)))

(defun dula-recursive-lean-assert (source &optional scope)
  "Register Lean assertions, compile SOURCE, and analyze rejected points."
  (let ((assertions (dula-lean-register-source source))
        (lean-result (dula-lean-run-source source)))
    (if (dula-lean-success-p lean-result)
        (list :status 'lean-accepted :lean lean-result :assertions assertions)
      (list :status 'lean-rejected
            :lean lean-result
            :assertions assertions
            :counterlemmas
            (mapcar (lambda (a) (dula-recursive-assert a scope)) assertions)))))

;;; Assertion-point API

(defmacro dula-assert-point (name assumptions conclusion &rest properties)
  "Declare a semantic assertion point.
ASSUMPTIONS is a list of proposition objects. CONCLUSION is a proposition."
  `(let ((a (dula-assertion-create ,name ,conclusion ,assumptions
                                   dula--current-source nil nil
                                   (list ,@properties))))
     (dula-recursive-assert a)
     a))

(defmacro dula-functor (name source target &rest body)
  "Declare a DULA functor binding."
  `(dula-functor-bind ,name ,source ,target ',body))

(defmacro dula-assertion (name proposition &rest assumptions)
  "Declare an assertion point with ASSUMPTIONS."
  `(dula-assert-point ,name ',assumptions ,proposition))

;;; Recursive Babel-compatible source markers

(defun dula-babel-assertion-marker-p (line)
  "Return non-nil when LINE is a DULA assertion marker."
  (string-match-p "^[[:space:]]*DULA-ASSERT:" line))

(defun dula-babel-counterlemma-marker-p (line)
  "Return non-nil when LINE is a DULA counterlemma marker."
  (string-match-p "^[[:space:]]*DULA-COUNTERLEMMA:" line))

(defun dula-babel-functor-marker-p (line)
  "Return non-nil when LINE is a DULA functor marker."
  (string-match-p "^[[:space:]]*DULA-FUNCTOR:" line))

(defun dula-babel-parse-marker (line)
  "Parse a DULA marker LINE into a property list."
  (cond
   ((dula-babel-assertion-marker-p line)
    (list :kind 'assertion :text (string-trim (string-remove-prefix "DULA-ASSERT:" (string-trim line)))))
   ((dula-babel-counterlemma-marker-p line)
    (list :kind 'counterlemma :text (string-trim (string-remove-prefix "DULA-COUNTERLEMMA:" (string-trim line)))))
   ((dula-babel-functor-marker-p line)
    (list :kind 'functor :text (string-trim (string-remove-prefix "DULA-FUNCTOR:" (string-trim line)))))
   (t nil)))

(defun dula-babel-scan (source)
  "Scan SOURCE for recursive DULA assertion markers."
  (let (markers)
    (dolist (line (split-string source "\n"))
      (let ((marker (dula-babel-parse-marker line)))
        (when marker (push marker markers))))
    (nreverse markers)))

(defun dula-babel-assertion-point (name proposition &optional assumptions)
  "Create an assertion point usable at a recursive source boundary."
  (dula-assertion-create name proposition assumptions dula--current-source nil nil
                         (list :assertion-point t :recursive t)))

(defun dula-babel-run-assertion-point (assertion &optional scope)
  "Run Alloy at ASSERTION's source assertion point."
  (dula-recursive-assert assertion scope))

;;; Serialization

(defun dula-proposition->plist (p)
  "Serialize proposition P."
  (list :id (dula-proposition-id p)
        :name (dula-proposition-name p)
        :kind (dula-proposition-kind p)
        :type (dula-proposition-type p)
        :source (dula-proposition-source p)
        :metadata (dula-proposition-metadata p)))

(defun dula-functor->plist (f)
  "Serialize functor F."
  (list :id (dula-functor-id f)
        :name (dula-functor-name f)
        :source (dula-functor-source f)
        :target (dula-functor-target f)
        :arity (dula-functor-arity f)
        :variance (dula-functor-variance f)
        :body (dula-functor-body f)
        :metadata (dula-functor-metadata f)))

(defun dula-assertion->plist (a)
  "Serialize assertion A."
  (list :id (dula-assertion-id a)
        :name (dula-assertion-name a)
        :proposition (dula-proposition-id (dula-assertion-proposition a))
        :assumptions (mapcar #'dula-proposition-id (dula-assertion-assumptions a))
        :source (dula-assertion-source a)
        :line (dula-assertion-line a)
        :column (dula-assertion-column a)
        :status (dula-assertion-status a)
        :depth (dula-assertion-depth a)
        :metadata (dula-assertion-metadata a)))

(defun dula-counterlemma->plist (c)
  "Serialize counterlemma C."
  (list :id (dula-counterlemma-id c)
        :assertion (dula-counterlemma-assertion c)
        :scope (dula-counterlemma-scope c)
        :formula (dula-proposition-expression (dula-counterlemma-formula c))
        :status (dula-counterlemma-status c)
        :alloy-file (dula-counterlemma-alloy-file c)
        :created-at (dula-counterlemma-created-at c)))

(defun dula-json-encode (object)
  "Encode OBJECT as JSON."
  (json-encode object))

(defun dula-export-state (&optional file)
  "Export DULA state to FILE or return JSON text."
  (let ((object
         (list :schema dula-schema-version
               :version dula-version
               :propositions (mapcar #'dula-proposition->plist
                                      (dula--hash-values dula--propositions))
               :functors (mapcar #'dula-functor->plist
                                  (dula--hash-values dula--functors))
               :assertions (mapcar #'dula-assertion->plist
                                    (dula--hash-values dula--assertions)))))
    (if file (dula--write-file file (dula-json-encode object))
      (dula-json-encode object))))

;;; Reports

(defun dula-status-symbol (status)
  "Render STATUS for humans."
  (pcase status
    ('counterexample-found "COUNTEREXAMPLE_FOUND")
    ('no-counterexample-found "NO_COUNTEREXAMPLE_WITHIN_SCOPE")
    ('lean-rejected "LEAN_REJECTED")
    ('lean-accepted "LEAN_ACCEPTED")
    (_ (upcase (symbol-name status)))))

(defun dula-report-counterlemma (c)
  "Render COUNTERLEMMA as a report."
  (format "DULA COUNTERLEMMA\nID: %s\nStatus: %s\nScope: %d\nFormula: %s\nAlloy: %s\nCreated: %s\n"
          (dula-counterlemma-id c)
          (dula-status-symbol (dula-counterlemma-status c))
          (dula-counterlemma-scope c)
          (dula-proposition-expression (dula-counterlemma-formula c))
          (or (dula-counterlemma-alloy-file c) "none")
          (dula-counterlemma-created-at c)))

(defun dula-report-assertion (a)
  "Render ASSERTION as a report."
  (format "ASSERTION %s\nStatus: %s\nDepth: %d\nSource line: %s\nAlloy: %s\n"
          (dula-assertion-name a)
          (dula-status-symbol (dula-assertion-status a))
          (dula-assertion-depth a)
          (or (dula-assertion-line a) "unknown")
          (or (dula-assertion-alloy-file a) "none")))

(defun dula-report-all ()
  "Return a report for all assertions."
  (mapconcat #'dula-report-assertion
             (dula--hash-values dula--assertions) "\n"))

;;; Interactive commands

(defun dula-analyze-current-buffer (&optional scope)
  "Analyze Lean declarations in the current buffer."
  (interactive)
  (let ((source (buffer-substring-no-properties (point-min) (point-max))))
    (message "%s" (dula-recursive-lean-assert source scope))))

(defun dula-analyze-region (beg end &optional scope)
  "Analyze Lean declarations in region BEG END."
  (interactive "r")
  (dula-recursive-lean-assert
   (buffer-substring-no-properties beg end) scope))

(defun dula-analyze-assertion (name &optional scope)
  "Analyze registered assertion NAME."
  (interactive "sAssertion name: ")
  (let ((a (dula-assertion-by-name name)))
    (unless a (user-error "No assertion named %s" name))
    (dula-recursive-assert a scope)))

(defun dula-show-last-result ()
  "Display the last DULA counterlemma."
  (interactive)
  (if dula--last-result
      (message "%s" (dula-report-counterlemma dula--last-result))
    (message "No DULA result")))

(defun dula-write-last-counterlemma ()
  "Write the last counterlemma's Lean witness target."
  (interactive)
  (unless dula--last-result (user-error "No counterlemma available"))
  (message "%s" (dula-counterlemma-write-lean dula--last-result)))

;;; Assertion semantics helpers

(defun dula-assumptions-and (assertion)
  "Return semantic conjunction for ASSERTION assumptions."
  (let ((items (dula-assertion-assumptions assertion)))
    (cond ((null items) (dula-proposition-atom "True" 'Prop))
          ((= 1 (length items)) (car items))
          (t (apply #'dula-proposition-and items)))))

(defun dula-assertion-semantics (assertion)
  "Return implication semantics of ASSERTION."
  (dula-proposition-implies (dula-assumptions-and assertion)
                            (dula-assertion-proposition assertion)))

(defun dula-assertion-negation (assertion)
  "Return negation of ASSERTION semantics."
  (dula-proposition-not (dula-assertion-semantics assertion)))

(defun dula-assertion-valid-shape-p (assertion)
  "Check structural validity of ASSERTION."
  (and (dula-assertion-p assertion)
       (dula-proposition-p (dula-assertion-proposition assertion))))

(defun dula-functor-valid-p (functor)
  "Check structural validity of FUNCTOR."
  (and (dula-functor-p functor)
       (dula--nonempty (dula-functor-name functor))
       (dula-functor-source functor)
       (dula-functor-target functor)))

(defun dula-proposition-valid-p (proposition)
  "Check structural validity of PROPOSITION."
  (and (dula-proposition-p proposition)
       (dula--nonempty (dula-proposition-name proposition))
       (dula-proposition-kind proposition)))

;;; Node graph

(defun dula-node-create (kind payload &optional parents depth metadata)
  "Create a graph NODE."
  (let ((node (make-dula-node :id (dula--uuid "node")
                              :kind kind :payload payload
                              :parents parents :children nil
                              :depth (or depth 0) :metadata metadata)))
    (puthash (dula-node-id node) node dula--nodes)
    (dolist (parent-id parents)
      (let ((parent (gethash parent-id dula--nodes)))
        (when parent
          (push (dula-node-id node) (dula-node-children parent)))))
    node))

(defun dula-node-get (id)
  "Return graph node ID."
  (gethash id dula--nodes))

(defun dula-node-children-of (node)
  "Return children of NODE."
  (mapcar #'dula-node-get (dula-node-children node)))

(defun dula-node-parents-of (node)
  "Return parents of NODE."
  (mapcar #'dula-node-get (dula-node-parents node)))

(defun dula-node-walk (node function &optional seen)
  "Walk NODE recursively with FUNCTION."
  (let ((seen (or seen (make-hash-table :test #'equal))))
    (unless (gethash (dula-node-id node) seen)
      (puthash (dula-node-id node) t seen)
      (funcall function node)
      (dolist (child (dula-node-children-of node))
        (when child (dula-node-walk child function seen))))))

;;; Lean diagnostic normalization

(defun dula-lean-json-message-p (line)
  "Return non-nil when LINE appears to be a Lean JSON message."
  (string-prefix-p "{" (string-trim-left line)))

(defun dula-lean-parse-message (line)
  "Parse a Lean JSON diagnostic LINE."
  (when (dula-lean-json-message-p line)
    (condition-case nil
        (json-parse-string line :object-type 'alist :null-object nil)
      (error nil))))

(defun dula-lean-message-severity (message)
  "Return severity from Lean MESSAGE."
  (alist-get 'severity message))

(defun dula-lean-message-data (message)
  "Return text from Lean MESSAGE."
  (alist-get 'data message))

(defun dula-lean-message-error-p (message)
  "Return non-nil when Lean MESSAGE is an error."
  (equal (alist-get 'severity message) "error"))

(defun dula-lean-parse-output (output)
  "Parse all JSON Lean messages from OUTPUT."
  (delq nil (mapcar #'dula-lean-parse-message (split-string output "\n" t))))

(defun dula-lean-result-errors (result)
  "Return parsed errors from RESULT."
  (seq-filter #'dula-lean-message-error-p (dula-lean-parse-output (cdr result))))

;;; Conservative proposition normalization

(defun dula-normalize-lean-proposition (text)
  "Normalize common Lean logical notation in TEXT.
This is textual normalization, not a Lean parser."
  (let ((s (string-trim text)))
    (setq s (replace-regexp-in-string "\\bTrue\\b" "True" s))
    (setq s (replace-regexp-in-string "\\bFalse\\b" "False" s))
    (setq s (replace-regexp-in-string "&&" " and " s))
    (setq s (replace-regexp-in-string "||" " or " s))
    (setq s (replace-regexp-in-string "→" " -> " s))
    (setq s (replace-regexp-in-string "↔" " <-> " s))
    (replace-regexp-in-string "¬" "not " s)))

(defun dula-proposition-from-lean (name text &optional source)
  "Create a proposition from a Lean proposition TEXT."
  (dula-proposition-create name 'lean type source
                           (list :lean-text (dula-normalize-lean-proposition text))))

;;; Alloy source inspection

(defun dula-alloy-model-valid-p (source)
  "Perform lightweight structural checks on Alloy SOURCE."
  (and (string-match-p "\\bsig\\b" source)
       (string-match-p "\\bcheck\\b" source)
       (string-match-p "\\bassert\\b" source)))

(defun dula-alloy-count-checks (source)
  "Count Alloy check commands in SOURCE."
  (let ((count 0) (start 0))
    (while (string-match "\\bcheck\\b" source start)
      (setq count (1+ count) start (match-end 0)))
    count))

(defun dula-alloy-count-predicates (source)
  "Count Alloy predicate declarations in SOURCE."
  (let ((count 0) (start 0))
    (while (string-match "\\bpred[[:space:]]+" source start)
      (setq count (1+ count) start (match-end 0)))
    count))

(defun dula-alloy-extract-run-lines (source)
  "Extract Alloy run command lines."
  (seq-filter (lambda (line) (string-match-p "^[[:space:]]*run\\b" line))
              (split-string source "\n" t)))

(defun dula-alloy-extract-check-lines (source)
  "Extract Alloy check command lines."
  (seq-filter (lambda (line) (string-match-p "^[[:space:]]*check\\b" line))
              (split-string source "\n" t)))

;;; Counterlemma witness parsing

(defun dula-counterlemma-witness-lines (counterlemma)
  "Return likely witness lines from COUNTERLEMMA output."
  (seq-filter
   (lambda (line)
     (or (string-match-p "^[[:space:]]*[A-Za-z0-9_]+[[:space:]]=" line)
         (string-match-p "State" line)
         (string-match-p "Instance" line)))
   (split-string (dula-counterlemma-witness counterlemma) "\n" t)))

(defun dula-counterlemma-has-witness-p (counterlemma)
  "Return non-nil when COUNTERLEMMA contains witness-looking output."
  (and (dula-counterlemma-witness counterlemma)
       (> (length (dula-counterlemma-witness-lines counterlemma)) 0)))

(defun dula-counterlemma-bounded-p (counterlemma)
  "All DULA Alloy counterlemmas are bounded by definition."
  (and (integerp (dula-counterlemma-scope counterlemma))
       (> (dula-counterlemma-scope counterlemma) 0)))

;;; Functor graph operations

(defun dula-functor-source-equal-p (left right)
  "Return whether LEFT target matches RIGHT source."
  (equal (dula-functor-target left) (dula-functor-source right)))

(defun dula-functor-compose-many (functors)
  "Compose FUNCTORS from right to left."
  (if (null functors) nil
    (let ((result (car (last functors))))
      (dolist (f (reverse (butlast functors)) result)
        (setq result (dula-functor-compose result f))))))

(defun dula-functor-map-all (functor propositions)
  "Map FUNCTOR across PROPOSITIONS."
  (mapcar (lambda (p) (dula-functor-map functor p)) propositions))

(defun dula-functor-laws (functor)
  "Return identity and composition law propositions for FUNCTOR."
  (list (dula-functor-law-identity functor)))

;;; Assertions over functor bindings

(defun dula-functor-assertion (functor proposition)
  "Create an assertion that FUNCTOR preserves PROPOSITION."
  (let ((mapped (dula-functor-map functor proposition)))
    (dula-assertion-create
     (format "%s_preserves_%s" (dula-functor-name functor)
             (dula-proposition-name proposition))
     mapped (list proposition) nil nil nil
     (list :functor (dula-functor-id functor)))))

(defun dula-functor-recursive-assert (functor propositions &optional scope)
  "Recursively assert FUNCTOR across PROPOSITIONS."
  (dolist (p propositions)
    (dula-recursive-assert (dula-functor-assertion functor p) scope)))

;;; Source artifact helpers

(defun dula-source-hash (source)
  "Return SHA-256 SOURCE hash when available."
  (secure-hash 'sha256 source))

(defun dula-source-artifact (source)
  "Return metadata describing SOURCE."
  (list :hash (dula-source-hash source)
        :bytes (string-bytes source)
        :lines (length (split-string source "\n"))))

(defun dula-assertion-artifact (assertion)
  "Return source metadata for ASSERTION."
  (dula-source-artifact (or (dula-assertion-source assertion) "")))

(defun dula-counterlemma-artifact (counterlemma)
  "Return metadata for COUNTERLEMMA."
  (list :id (dula-counterlemma-id counterlemma)
        :scope (dula-counterlemma-scope counterlemma)
        :bounded t
        :status (dula-counterlemma-status counterlemma)
        :formula (dula-proposition-expression (dula-counterlemma-formula counterlemma))))

;;; Recursive dependency helpers

(defun dula-assertion-parent-ids (assertion)
  "Return parent assertion IDs."
  (plist-get (dula-assertion-metadata assertion) :parents))

(defun dula-assertion-set-parents (assertion parents)
  "Set ASSERTION parent IDs to PARENTS."
  (setf (dula-assertion-metadata assertion)
        (plist-put (dula-assertion-metadata assertion) :parents parents))
  assertion)

(defun dula-assertion-add-parent (assertion parent)
  "Add PARENT as an assertion dependency."
  (dula-assertion-set-parents
   assertion
   (delete-dups (cons (dula-assertion-id parent)
                      (dula-assertion-parent-ids assertion)))))

(defun dula-assertion-dependency-cycle-p (assertion)
  "Detect recursive dependency cycle beginning at ASSERTION."
  (let ((target (dula-assertion-id assertion))
        (seen (make-hash-table :test #'equal)))
    (cl-labels ((walk (id)
                  (cond ((equal id target) t)
                        ((gethash id seen) nil)
                        (t (puthash id t seen)
                           (let ((a (dula-assertion-get id)))
                             (and a (seq-some #'walk (dula-assertion-parent-ids a))))))))
      (seq-some #'walk (dula-assertion-parent-ids assertion)))))

;;; Bounded recursive search policies

(defun dula-scope-sequence (&optional start stop step)
  "Return integer scopes from START through STOP by STEP."
  (let ((start (or start 1)) (stop (or stop dula-default-scope))
        (step (or step 1)) result)
    (cl-loop for n from start to stop by step do (push n result))
    (nreverse result)))

(defun dula-search-scopes (assertion scopes)
  "Run ASSERTION over SCOPES and return counterlemma results."
  (mapcar (lambda (scope) (dula-recursive-assert assertion scope)) scopes))

(defun dula-first-counterexample (results)
  "Return first counterexample-found result from RESULTS."
  (seq-find (lambda (c) (eq (dula-counterlemma-status c) 'counterexample-found)) results))

(defun dula-all-scopes-clean-p (results)
  "Return non-nil when no bounded scope produced a counterexample."
  (not (dula-first-counterexample results)))

;;; Generated Lean assertion targets

(defun dula-counterlemma-name (counterlemma)
  "Return a Lean-safe counterlemma name."
  (format "dula_counterlemma_%s" (dula-counterlemma-id counterlemma)))

(defun dula-counterlemma-target (counterlemma)
  "Return the Lean target text for COUNTERLEMMA."
  (format "theorem %s : False := by\n  sorry\n"
          (dula-counterlemma-name counterlemma)))

(defun dula-counterlemma-comment (counterlemma)
  "Return provenance comment for COUNTERLEMMA."
  (format "-- DULA: bounded Alloy countermodel, scope=%d, status=%s\n"
          (dula-counterlemma-scope counterlemma)
          (dula-status-symbol (dula-counterlemma-status counterlemma))))

(defun dula-counterlemma-lean-artifact (counterlemma)
  "Return complete Lean-facing artifact for COUNTERLEMMA."
  (concat (dula-counterlemma-comment counterlemma)
          (dula-counterlemma-target counterlemma)))

;;; Assertion-point reports

(defun dula-assertion-point-report (assertion)
  "Return machine-readable assertion point report."
  (list :id (dula-assertion-id assertion)
        :name (dula-assertion-name assertion)
        :line (dula-assertion-line assertion)
        :column (dula-assertion-column assertion)
        :status (dula-assertion-status assertion)
        :alloy-file (dula-assertion-alloy-file assertion)
        :depth (dula-assertion-depth assertion)))

(defun dula-assertion-point-comment (assertion)
  "Return a source comment identifying ASSERTION."
  (format "-- DULA-ASSERT: %s [%s]\n"
          (dula-assertion-name assertion)
          (dula-assertion-id assertion)))

;;; Library manifest

(defun dula-manifest ()
  "Return DULA library manifest."
  (list :name "DULA Lean Alloy"
        :version dula-version
        :schema dula-schema-version
        :lean dula-lean-command
        :alloy dula-alloy-command
        :scope dula-default-scope
        :recursive t
        :functor-bindings t
        :assertion-points t
        :babel-compatibility 'markers-only
        :babel-runtime nil))

(defun dula-capabilities ()
  "Return advertised DULA capabilities."
  '(semantic-propositions functor-bindings assertion-points
    alloy-countermodels counterlemmas recursive-assertions
    lean-diagnostic-capture artifact-provenance bounded-search))

(provide 'dula-lean-alloy)

;;; dula-lean-alloy.el ends here

(defun dula-detail-1 (&optional object)
  "Return diagnostic detail 1 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 1))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 1))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 1))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 1))
   (t (list :kind 'unknown :detail 1))))

(defun dula-detail-2 (&optional object)
  "Return diagnostic detail 2 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 2))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 2))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 2))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 2))
   (t (list :kind 'unknown :detail 2))))

(defun dula-detail-3 (&optional object)
  "Return diagnostic detail 3 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 3))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 3))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 3))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 3))
   (t (list :kind 'unknown :detail 3))))

(defun dula-detail-4 (&optional object)
  "Return diagnostic detail 4 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 4))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 4))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 4))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 4))
   (t (list :kind 'unknown :detail 4))))

(defun dula-detail-5 (&optional object)
  "Return diagnostic detail 5 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 5))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 5))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 5))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 5))
   (t (list :kind 'unknown :detail 5))))

(defun dula-detail-6 (&optional object)
  "Return diagnostic detail 6 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 6))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 6))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 6))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 6))
   (t (list :kind 'unknown :detail 6))))

(defun dula-detail-7 (&optional object)
  "Return diagnostic detail 7 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 7))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 7))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 7))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 7))
   (t (list :kind 'unknown :detail 7))))

(defun dula-detail-8 (&optional object)
  "Return diagnostic detail 8 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 8))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 8))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 8))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 8))
   (t (list :kind 'unknown :detail 8))))

(defun dula-detail-9 (&optional object)
  "Return diagnostic detail 9 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 9))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 9))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 9))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 9))
   (t (list :kind 'unknown :detail 9))))

(defun dula-detail-10 (&optional object)
  "Return diagnostic detail 10 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 10))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 10))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 10))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 10))
   (t (list :kind 'unknown :detail 10))))

(defun dula-detail-11 (&optional object)
  "Return diagnostic detail 11 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 11))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 11))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 11))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 11))
   (t (list :kind 'unknown :detail 11))))

(defun dula-detail-12 (&optional object)
  "Return diagnostic detail 12 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 12))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 12))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 12))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 12))
   (t (list :kind 'unknown :detail 12))))

(defun dula-detail-13 (&optional object)
  "Return diagnostic detail 13 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 13))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 13))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 13))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 13))
   (t (list :kind 'unknown :detail 13))))

(defun dula-detail-14 (&optional object)
  "Return diagnostic detail 14 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 14))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 14))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 14))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 14))
   (t (list :kind 'unknown :detail 14))))

(defun dula-detail-15 (&optional object)
  "Return diagnostic detail 15 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 15))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 15))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 15))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 15))
   (t (list :kind 'unknown :detail 15))))

(defun dula-detail-16 (&optional object)
  "Return diagnostic detail 16 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 16))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 16))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 16))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 16))
   (t (list :kind 'unknown :detail 16))))

(defun dula-detail-17 (&optional object)
  "Return diagnostic detail 17 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 17))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 17))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 17))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 17))
   (t (list :kind 'unknown :detail 17))))

(defun dula-detail-18 (&optional object)
  "Return diagnostic detail 18 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 18))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 18))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 18))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 18))
   (t (list :kind 'unknown :detail 18))))

(defun dula-detail-19 (&optional object)
  "Return diagnostic detail 19 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 19))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 19))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 19))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 19))
   (t (list :kind 'unknown :detail 19))))

(defun dula-detail-20 (&optional object)
  "Return diagnostic detail 20 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 20))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 20))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 20))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 20))
   (t (list :kind 'unknown :detail 20))))

(defun dula-detail-21 (&optional object)
  "Return diagnostic detail 21 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 21))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 21))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 21))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 21))
   (t (list :kind 'unknown :detail 21))))

(defun dula-detail-22 (&optional object)
  "Return diagnostic detail 22 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 22))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 22))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 22))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 22))
   (t (list :kind 'unknown :detail 22))))

(defun dula-detail-23 (&optional object)
  "Return diagnostic detail 23 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 23))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 23))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 23))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 23))
   (t (list :kind 'unknown :detail 23))))

(defun dula-detail-24 (&optional object)
  "Return diagnostic detail 24 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 24))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 24))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 24))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 24))
   (t (list :kind 'unknown :detail 24))))

(defun dula-detail-25 (&optional object)
  "Return diagnostic detail 25 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 25))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 25))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 25))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 25))
   (t (list :kind 'unknown :detail 25))))

(defun dula-detail-26 (&optional object)
  "Return diagnostic detail 26 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 26))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 26))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 26))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 26))
   (t (list :kind 'unknown :detail 26))))

(defun dula-detail-27 (&optional object)
  "Return diagnostic detail 27 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 27))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 27))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 27))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 27))
   (t (list :kind 'unknown :detail 27))))

(defun dula-detail-28 (&optional object)
  "Return diagnostic detail 28 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 28))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 28))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 28))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 28))
   (t (list :kind 'unknown :detail 28))))

(defun dula-detail-29 (&optional object)
  "Return diagnostic detail 29 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 29))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 29))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 29))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 29))
   (t (list :kind 'unknown :detail 29))))

(defun dula-detail-30 (&optional object)
  "Return diagnostic detail 30 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 30))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 30))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 30))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 30))
   (t (list :kind 'unknown :detail 30))))

(defun dula-detail-31 (&optional object)
  "Return diagnostic detail 31 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 31))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 31))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 31))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 31))
   (t (list :kind 'unknown :detail 31))))

(defun dula-detail-32 (&optional object)
  "Return diagnostic detail 32 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 32))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 32))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 32))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 32))
   (t (list :kind 'unknown :detail 32))))

(defun dula-detail-33 (&optional object)
  "Return diagnostic detail 33 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 33))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 33))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 33))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 33))
   (t (list :kind 'unknown :detail 33))))

(defun dula-detail-34 (&optional object)
  "Return diagnostic detail 34 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 34))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 34))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 34))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 34))
   (t (list :kind 'unknown :detail 34))))

(defun dula-detail-35 (&optional object)
  "Return diagnostic detail 35 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 35))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 35))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 35))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 35))
   (t (list :kind 'unknown :detail 35))))

(defun dula-detail-36 (&optional object)
  "Return diagnostic detail 36 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 36))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 36))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 36))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 36))
   (t (list :kind 'unknown :detail 36))))

(defun dula-detail-37 (&optional object)
  "Return diagnostic detail 37 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 37))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 37))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 37))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 37))
   (t (list :kind 'unknown :detail 37))))

(defun dula-detail-38 (&optional object)
  "Return diagnostic detail 38 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 38))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 38))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 38))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 38))
   (t (list :kind 'unknown :detail 38))))

(defun dula-detail-39 (&optional object)
  "Return diagnostic detail 39 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 39))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 39))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 39))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 39))
   (t (list :kind 'unknown :detail 39))))

(defun dula-detail-40 (&optional object)
  "Return diagnostic detail 40 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 40))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 40))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 40))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 40))
   (t (list :kind 'unknown :detail 40))))

(defun dula-detail-41 (&optional object)
  "Return diagnostic detail 41 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 41))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 41))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 41))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 41))
   (t (list :kind 'unknown :detail 41))))

(defun dula-detail-42 (&optional object)
  "Return diagnostic detail 42 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 42))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 42))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 42))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 42))
   (t (list :kind 'unknown :detail 42))))

(defun dula-detail-43 (&optional object)
  "Return diagnostic detail 43 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 43))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 43))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 43))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 43))
   (t (list :kind 'unknown :detail 43))))

(defun dula-detail-44 (&optional object)
  "Return diagnostic detail 44 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 44))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 44))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 44))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 44))
   (t (list :kind 'unknown :detail 44))))

(defun dula-detail-45 (&optional object)
  "Return diagnostic detail 45 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 45))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 45))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 45))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 45))
   (t (list :kind 'unknown :detail 45))))

(defun dula-detail-46 (&optional object)
  "Return diagnostic detail 46 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 46))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 46))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 46))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 46))
   (t (list :kind 'unknown :detail 46))))

(defun dula-detail-47 (&optional object)
  "Return diagnostic detail 47 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 47))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 47))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 47))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 47))
   (t (list :kind 'unknown :detail 47))))

(defun dula-detail-48 (&optional object)
  "Return diagnostic detail 48 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 48))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 48))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 48))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 48))
   (t (list :kind 'unknown :detail 48))))

(defun dula-detail-49 (&optional object)
  "Return diagnostic detail 49 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 49))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 49))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 49))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 49))
   (t (list :kind 'unknown :detail 49))))

(defun dula-detail-50 (&optional object)
  "Return diagnostic detail 50 for OBJECT without mutating DULA state."
  (cond
   ((dula-assertion-p object)
    (list :kind 'assertion :id (dula-assertion-id object)
          :name (dula-assertion-name object) :status (dula-assertion-status object)
          :detail 50))
   ((dula-counterlemma-p object)
    (list :kind 'counterlemma :id (dula-counterlemma-id object)
          :status (dula-counterlemma-status object) :scope (dula-counterlemma-scope object)
          :detail 50))
   ((dula-proposition-p object)
    (list :kind 'proposition :id (dula-proposition-id object)
          :kind-value (dula-proposition-kind object) :detail 50))
   ((dula-functor-p object)
    (list :kind 'functor :id (dula-functor-id object)
          :name (dula-functor-name object) :detail 50))
   (t (list :kind 'unknown :detail 50))))

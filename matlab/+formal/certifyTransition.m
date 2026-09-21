function certificate = certifyTransition(F, valid, admissible, invariant, S, x)
%certifyTransition Verify that a state transition satisfies the certification contract.
%
% A transition is certified exactly when:
%   1. Source state S is valid: Valid(S) = true
%   2. Input x is admissible: Admissible(x) = true
%   3. Target state S' = F(S,x) satisfies the invariant: I(F(S,x)) = true
%
% The target is computed definitionally as F(S,x), not caller-supplied.
%
% Usage:
%   certificate = formal.certifyTransition(F, valid, admissible, invariant, S, x)
%
% Returns:
%   certificate - struct with fields:
%       source              - input state S
%       input               - input x
%       target              - computed state S' = F(S,x)
%       sourceValid         - logical: Valid(S)
%       inputAdmissible     - logical: Admissible(x)
%       targetInvariant     - logical: I(F(S,x))
%       certified           - logical: sourceValid AND inputAdmissible AND targetInvariant
%
% Example:
%   F = @(S, x) S + x;
%   valid = @(S) S >= 0 && floor(S) == S;
%   admissible = @(x) x >= 0 && x <= 2;
%   invariant = @(S) S >= 0;
%
%   cert = formal.certifyTransition(F, valid, admissible, invariant, 3, 2);
%   if cert.certified
%       disp('Transition 3 + 2 = 5 is certified.');
%   end

arguments
    F (1,1) function_handle
    valid (1,1) function_handle
    admissible (1,1) function_handle
    invariant (1,1) function_handle
    S
    x
end

% Compute target state definitionally
Snext = F(S, x);

% Validate predicates return logical scalars
sourceValid = mustReturnLogicalScalar(valid(S), ...
    "valid(S) must return a logical scalar.");

inputAdmissible = mustReturnLogicalScalar(admissible(x), ...
    "admissible(x) must return a logical scalar.");

targetInvariant = mustReturnLogicalScalar(invariant(Snext), ...
    "invariant(F(S,x)) must return a logical scalar.");

% Construct certificate
certificate = struct( ...
    "source", S, ...
    "input", x, ...
    "target", Snext, ...
    "sourceValid", sourceValid, ...
    "inputAdmissible", inputAdmissible, ...
    "targetInvariant", targetInvariant, ...
    "certified", sourceValid && inputAdmissible && targetInvariant);

end

function value = mustReturnLogicalScalar(value, message)
%mustReturnLogicalScalar Enforce that a predicate returns a logical scalar.
%
% Predicate functions must return a true logical scalar, not:
%   - A numeric 1 (not islogical)
%   - A vector or matrix
%   - An array
%
% This strengthens contract verification: implicit numeric-to-boolean
% coercion is not permitted in certification predicates.

if ~(islogical(value) && isscalar(value))
    error("formal:InvalidPredicateResult", "%s", message);
end

end

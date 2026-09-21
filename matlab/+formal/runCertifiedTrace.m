function states = runCertifiedTrace(F, valid, admissible, invariant, S0, inputs)
%runCertifiedTrace Execute a state transition sequence, halting on first uncertified step.
%
% Unlike certifyTrace, which generates the complete trace then classifies it,
% runCertifiedTrace stops immediately when certification fails. This enforces
% the operational reading: "a rejected transition cannot enter the state sequence."
%
% Usage:
%   states = formal.runCertifiedTrace(F, valid, admissible, invariant, S0, inputs)
%
% Inputs:
%   F              - function_handle: (S, x) -> S' (state transition)
%   valid          - function_handle: S -> logical (source validity predicate)
%   admissible     - function_handle: x -> logical (input admissibility predicate)
%   invariant      - function_handle: S -> logical (target invariant predicate)
%   S0             - initial state
%   inputs         - cell array of inputs {x1, x2, ..., xn}
%
% Returns:
%   states - cell array {S0, S1, ..., Sk} of states up to (and including)
%            the last certified transition. If the first step fails, returns {S0}.
%
% Throws:
%   error("formal:RejectedTransition", ...) if any step is not certified.
%
% Example:
%   F = @(S, x) S + x;
%   valid = @(S) S >= 0 && floor(S) == S;
%   admissible = @(x) x >= 0 && x <= 2;
%   invariant = @(S) S >= 0;
%
%   try
%       states = formal.runCertifiedTrace(F, valid, admissible, invariant, 0, {1, 3, 1});
%   catch ME
%       disp(ME.message);  % Transition 2 was not certified.
%   end

arguments
    F (1,1) function_handle
    valid (1,1) function_handle
    admissible (1,1) function_handle
    invariant (1,1) function_handle
    S0
    inputs cell
end

states = cell(numel(inputs) + 1, 1);
states{1} = S0;

for r = 1:numel(inputs)
    certificate = formal.certifyTransition( ...
        F, valid, admissible, invariant, states{r}, inputs{r});

    if ~certificate.certified
        error("formal:RejectedTransition", ...
            "Transition %d was not certified.", r);
    end

    states{r + 1} = certificate.target;
end

end

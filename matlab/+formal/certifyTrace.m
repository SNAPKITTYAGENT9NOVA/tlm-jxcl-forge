function traceCertificate = certifyTrace(F, valid, admissible, invariant, S0, inputs)
%certifyTrace Verify that an entire execution trace satisfies the certification contract.
%
% Applies certifyTransition to each step of the trace sequentially.
% The complete trace is certified only if every step is certified.
%
% Important: This function generates the entire trace even if a step fails
% certification. Use runCertifiedTrace instead to halt on first failure.
%
% Usage:
%   traceCertificate = formal.certifyTrace(F, valid, admissible, invariant, S0, inputs)
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
%   traceCertificate - struct with fields:
%       initialState   - initial state S0
%       inputs         - cell array of inputs supplied
%       states         - cell array {S0, S1, S2, ..., Sn} of all states
%       steps          - cell array of certification structs from certifyTransition
%       certified      - logical: true if all steps are certified
%
% Example:
%   F = @(S, x) S + x;
%   valid = @(S) S >= 0 && floor(S) == S;
%   admissible = @(x) x >= 0 && x <= 2;
%   invariant = @(S) S >= 0;
%
%   trace = formal.certifyTrace(F, valid, admissible, invariant, 0, {1, 2, 1});
%   if trace.certified
%       disp('Entire trace is certified.');
%       disp(trace.states);  % {0; 1; 3; 4}
%   else
%       disp('Trace contains uncertified steps.');
%   end

arguments
    F (1,1) function_handle
    valid (1,1) function_handle
    admissible (1,1) function_handle
    invariant (1,1) function_handle
    S0
    inputs cell
end

numSteps = numel(inputs);
stepCertificates = cell(numSteps, 1);
states = cell(numSteps + 1, 1);
states{1} = S0;

% Process each step sequentially
for r = 1:numSteps
    currentState = states{r};
    currentInput = inputs{r};

    % Certify this transition
    stepCertificates{r} = formal.certifyTransition( ...
        F, valid, admissible, invariant, currentState, currentInput);

    % Advance to next state (regardless of certification status)
    states{r + 1} = stepCertificates{r}.target;
end

% Trace is certified only if every step is certified
isCertified = all(cellfun(@(c) c.certified, stepCertificates));

% Construct trace certificate
traceCertificate = struct( ...
    "initialState", S0, ...
    "inputs", {inputs}, ...
    "states", {states}, ...
    "steps", {stepCertificates}, ...
    "certified", isCertified);

end

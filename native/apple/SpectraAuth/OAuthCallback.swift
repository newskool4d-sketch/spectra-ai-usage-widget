import Foundation

public struct SpectraOAuthCallback: Sendable {
    public let code: String
    public let state: String

    public init(code: String, state: String) {
        self.code = code
        self.state = state
    }
}

public enum SpectraOAuthCallbackError: Error {
    case invalidScheme
    case invalidCallbackTarget
    case fragmentNotAllowed
    case providerRejected
    case missingCode
    case duplicateCode
    case missingState
    case duplicateState
    case stateMismatch
}

public enum SpectraOAuthCallbackParser {
    public static func parse(
        _ url: URL,
        expectedState: String,
        scheme: String = "spectra"
    ) throws -> SpectraOAuthCallback {
        guard url.scheme?.lowercased() == scheme.lowercased() else {
            throw SpectraOAuthCallbackError.invalidScheme
        }
        guard url.host?.lowercased() == "oauth", url.path == "/callback" else {
            throw SpectraOAuthCallbackError.invalidCallbackTarget
        }
        guard url.fragment == nil else {
            throw SpectraOAuthCallbackError.fragmentNotAllowed
        }

        let items = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
        if items.contains(where: { $0.name == "error" }) {
            throw SpectraOAuthCallbackError.providerRejected
        }

        let codeValues = items.filter { $0.name == "code" }.compactMap(\.value)
        let stateValues = items.filter { $0.name == "state" }.compactMap(\.value)

        guard codeValues.count == 1 else {
            throw codeValues.isEmpty ? SpectraOAuthCallbackError.missingCode : SpectraOAuthCallbackError.duplicateCode
        }
        guard stateValues.count == 1 else {
            throw stateValues.isEmpty ? SpectraOAuthCallbackError.missingState : SpectraOAuthCallbackError.duplicateState
        }

        let code = codeValues[0]
        let state = stateValues[0]
        guard !code.isEmpty else { throw SpectraOAuthCallbackError.missingCode }
        guard !state.isEmpty else { throw SpectraOAuthCallbackError.missingState }
        guard state == expectedState else { throw SpectraOAuthCallbackError.stateMismatch }

        return SpectraOAuthCallback(code: code, state: state)
    }
}

import AuthenticationServices
import Foundation

public enum SpectraOAuthSessionError: Error {
    case alreadyRunning
    case authorizeURLMustUseHTTPS
    case missingPresentationAnchor
    case unableToStart
    case callbackMissing
}

@MainActor
public final class SpectraOAuthSessionCoordinator: NSObject, ASWebAuthenticationPresentationContextProviding {
    private var session: ASWebAuthenticationSession?
    private var presentationAnchor: ASPresentationAnchor?

    public override init() {
        super.init()
    }

    public init(presentationAnchor: ASPresentationAnchor) {
        self.presentationAnchor = presentationAnchor
        super.init()
    }

    public func setPresentationAnchor(_ anchor: ASPresentationAnchor) {
        presentationAnchor = anchor
    }

    public func start(
        authorizeURL: URL,
        callbackScheme: String = "spectra",
        expectedState: String,
        prefersEphemeralWebBrowserSession: Bool = false,
        completion: @escaping (Result<SpectraOAuthCallback, Error>) -> Void
    ) throws {
        guard session == nil else { throw SpectraOAuthSessionError.alreadyRunning }
        guard authorizeURL.scheme?.lowercased() == "https" else {
            throw SpectraOAuthSessionError.authorizeURLMustUseHTTPS
        }
        guard presentationAnchor != nil else {
            throw SpectraOAuthSessionError.missingPresentationAnchor
        }

        let authSession = ASWebAuthenticationSession(
            url: authorizeURL,
            callback: .customScheme(callbackScheme)
        ) { [weak self] callbackURL, error in
            guard let self else { return }
            self.session = nil

            if let error {
                completion(.failure(error))
                return
            }
            guard let callbackURL else {
                completion(.failure(SpectraOAuthSessionError.callbackMissing))
                return
            }

            do {
                let callback = try SpectraOAuthCallbackParser.parse(
                    callbackURL,
                    expectedState: expectedState,
                    scheme: callbackScheme
                )
                completion(.success(callback))
            } catch {
                completion(.failure(error))
            }
        }

        authSession.presentationContextProvider = self
        authSession.prefersEphemeralWebBrowserSession = prefersEphemeralWebBrowserSession
        session = authSession
        guard authSession.start() else {
            session = nil
            throw SpectraOAuthSessionError.unableToStart
        }
    }

    public func cancel() {
        session?.cancel()
        session = nil
    }

    public func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        guard let presentationAnchor else {
            preconditionFailure("SPECTRA OAuth presentation anchor was not configured")
        }
        return presentationAnchor
    }
}

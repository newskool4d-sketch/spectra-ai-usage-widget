import type { ProviderId } from "../data/providers";

export type StoredCredential = Readonly<{
  providerId: ProviderId;
  accessToken: string;
  refreshToken?: string;
  expiresAt?: string;
}>;

/**
 * The interface is the security boundary for Tauri/Swift implementations.
 * No browser storage fallback is provided by the prototype.
 */
export type CredentialVault = Readonly<{
  read: (providerId: ProviderId) => Promise<StoredCredential | null>;
  write: (credential: StoredCredential) => Promise<void>;
  remove: (providerId: ProviderId) => Promise<void>;
}>;

export const unavailableCredentialVault: CredentialVault = Object.freeze({
  read: async () => null,
  write: async () => undefined,
  remove: async () => undefined
});

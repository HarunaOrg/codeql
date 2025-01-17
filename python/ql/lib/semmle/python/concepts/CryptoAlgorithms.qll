/**
 * Provides classes modeling cryptographic algorithms, separated into strong and weak variants.
 *
 * The classification into strong and weak are based on Wikipedia, OWASP and google (2017).
 */

/**
 * Names of cryptographic algorithms, separated into strong and weak variants.
 *
 * The names are normalized: upper-case, no spaces, dashes or underscores.
 *
 * The names are inspired by the names used in real world crypto libraries.
 *
 * The classification into strong and weak are based on Wikipedia, OWASP and google (2017).
 */
private module AlgorithmNames {
  predicate isStrongHashingAlgorithm(string name) {
    name =
      [
        "DSA", "ED25519", "ES256", "ECDSA256", "ES384", "ECDSA384", "ES512", "ECDSA512", "SHA2",
        "SHA224", "SHA256", "SHA384", "SHA512", "SHA3", "SHA3224", "SHA3256", "SHA3384", "SHA3512"
      ]
  }

  predicate isWeakHashingAlgorithm(string name) {
    name =
      [
        "HAVEL128", "MD2", "MD4", "MD5", "PANAMA", "RIPEMD", "RIPEMD128", "RIPEMD256", "RIPEMD160",
        "RIPEMD320", "SHA0", "SHA1"
      ]
  }

  predicate isStrongEncryptionAlgorithm(string name) {
    name = ["AES", "AES128", "AES192", "AES256", "AES512", "RSA", "RABBIT", "BLOWFISH"]
  }

  predicate isWeakEncryptionAlgorithm(string name) {
    name =
      [
        "DES", "3DES", "TRIPLEDES", "TDEA", "TRIPLEDEA", "ARC2", "RC2", "ARC4", "RC4", "ARCFOUR",
        "ARC5", "RC5"
      ]
  }

  predicate isStrongPasswordHashingAlgorithm(string name) {
    name = ["ARGON2", "PBKDF2", "BCRYPT", "SCRYPT"]
  }

  predicate isWeakPasswordHashingAlgorithm(string name) { name = "EVPKDF" }
}

private import AlgorithmNames

/**
 * A cryptographic algorithm.
 */
private newtype TCryptographicAlgorithm =
  MkHashingAlgorithm(string name, boolean isWeak) {
    isStrongHashingAlgorithm(name) and isWeak = false
    or
    isWeakHashingAlgorithm(name) and isWeak = true
  } or
  MkEncryptionAlgorithm(string name, boolean isWeak) {
    isStrongEncryptionAlgorithm(name) and isWeak = false
    or
    isWeakEncryptionAlgorithm(name) and isWeak = true
  } or
  MkPasswordHashingAlgorithm(string name, boolean isWeak) {
    isStrongPasswordHashingAlgorithm(name) and isWeak = false
    or
    isWeakPasswordHashingAlgorithm(name) and isWeak = true
  }

/**
 * A cryptographic algorithm.
 */
abstract class CryptographicAlgorithm extends TCryptographicAlgorithm {
  /** Gets a textual representation of this element. */
  string toString() { result = getName() }

  /**
   * Gets the normalized name of this algorithm (upper-case, no spaces, dashes or underscores).
   */
  abstract string getName();

  /**
   * Holds if the name of this algorithm matches `name` modulo case,
   * white space, dashes, underscores, and anything after a dash in the name
   * (to ignore modes of operation, such as CBC or ECB).
   */
  bindingset[name]
  predicate matchesName(string name) {
    [name.toUpperCase(), name.toUpperCase().regexpCapture("^(\\w+)(?:-.*)?$", 1)]
        .regexpReplaceAll("[-_ ]", "") = getName()
  }

  /**
   * Holds if this algorithm is weak.
   */
  abstract predicate isWeak();
}

/**
 * A hashing algorithm such as `MD5` or `SHA512`.
 */
class HashingAlgorithm extends MkHashingAlgorithm, CryptographicAlgorithm {
  string name;
  boolean isWeak;

  HashingAlgorithm() { this = MkHashingAlgorithm(name, isWeak) }

  override string getName() { result = name }

  override predicate isWeak() { isWeak = true }
}

/**
 * An encryption algorithm such as `DES` or `AES512`.
 */
class EncryptionAlgorithm extends MkEncryptionAlgorithm, CryptographicAlgorithm {
  string name;
  boolean isWeak;

  EncryptionAlgorithm() { this = MkEncryptionAlgorithm(name, isWeak) }

  override string getName() { result = name }

  override predicate isWeak() { isWeak = true }
}

/**
 * A password hashing algorithm such as `PBKDF2` or `SCRYPT`.
 */
class PasswordHashingAlgorithm extends MkPasswordHashingAlgorithm, CryptographicAlgorithm {
  string name;
  boolean isWeak;

  PasswordHashingAlgorithm() { this = MkPasswordHashingAlgorithm(name, isWeak) }

  override string getName() { result = name }

  override predicate isWeak() { isWeak = true }
}

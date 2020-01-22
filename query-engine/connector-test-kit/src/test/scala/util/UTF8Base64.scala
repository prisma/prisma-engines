package util

import java.nio.charset.StandardCharsets
import java.util.Base64

object UTF8Base64 {
  // Important: Rust requires UTF-8 encoding (encodeToString uses Latin-1)

  def encode(input: String): String = {
    val encoded = Base64.getEncoder.encode(input.getBytes(StandardCharsets.UTF_8))
    new String(encoded, StandardCharsets.UTF_8)
  }

  def decode(input: String): String = {
    val decoded = Base64.getDecoder.decode(input.trim.getBytes(StandardCharsets.UTF_8))
    new String(decoded, StandardCharsets.UTF_8)
  }
}

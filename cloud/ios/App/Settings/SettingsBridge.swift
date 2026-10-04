// 调 Rust 桥读写 config.toml；JSON 进出，失败的原因是给用户看的中文。账号接口在 Account/AccountBridge.swift。

import Foundation
import QingjianBridge

enum SettingsBridge {
    static func readSettings(config: URL, dicts: URL) -> KeyboardSettings? {
        let raw = config.path.withCString { c in dicts.path.withCString { qj_settings_read(c, $0) } }
        return decode(take(raw))
    }

    /// 成功返回 nil，失败返回原因。
    static func writeSettings(_ settings: KeyboardSettings, config: URL) -> String? {
        guard let json = encode(settings) else { return "设置编码失败" }
        return take(config.path.withCString { c in json.withCString { qj_settings_write(c, $0) } })
    }

    /// 取走桥返回的字符串并释放。
    static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    static func decode<T: Decodable>(_ json: String?) -> T? {
        guard let data = json?.data(using: .utf8) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(T.self, from: data)
    }

    private static func encode<T: Encodable>(_ value: T) -> String? {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return (try? encoder.encode(value)).flatMap { String(data: $0, encoding: .utf8) }
    }
}

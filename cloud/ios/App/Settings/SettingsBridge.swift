// 调 Rust 桥读写 config.toml 与 cloud.toml；JSON 进出，失败的原因是给用户看的中文。

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

    static func readCloud(_ file: URL) -> CloudSettings? {
        decode(take(file.path.withCString { qj_cloud_config_read($0) }))
    }

    static func writeCloud(_ cloud: CloudSettings, file: URL) -> String? {
        guard let json = encode(cloud.switches) else { return "配置编码失败" }
        return take(file.path.withCString { f in json.withCString { qj_cloud_config_write(f, $0) } })
    }

    private static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    private static func decode<T: Decodable>(_ json: String?) -> T? {
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

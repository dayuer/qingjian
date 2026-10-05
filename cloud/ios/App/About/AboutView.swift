// 关于页：名字与版本、基于青简输入法的 GPL 署名、源码与许可证全文链接，MiSans 字体署名与随包的协议全文。

import SwiftUI

struct AboutView: View {
    var body: some View {
        Form {
            Section {
                LabeledContent(AboutInfo.name, value: AboutInfo.versionText(info: Bundle.main.infoDictionary))
            }
            Section {
                Text(AboutInfo.attribution)
                Link("源码", destination: AboutInfo.sourceURL)
                Link("GPL-3.0 许可证全文", destination: AboutInfo.licenseURL)
            } header: {
                Text("开源许可")
            }
            Section {
                Text(AboutInfo.fontAttribution)
                if let license = AboutInfo.fontLicenseText() {
                    NavigationLink("MiSans 字体许可协议") { FontLicenseView(text: license) }
                }
            } header: {
                Text("字体")
            }
        }
        .navigationTitle("关于")
    }
}

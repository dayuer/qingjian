// MiSans 字体许可协议全文（随包的 MiSans-LICENSE.txt），从关于页点进来看。

import SwiftUI

struct FontLicenseView: View {
    let text: String

    var body: some View {
        ScrollView {
            Text(text)
                .font(AppFont.footnote)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
        }
        .navigationTitle("MiSans 字体许可协议")
        .navigationBarTitleDisplayMode(.inline)
    }
}

// 一个候选（候选栏与展开面板共用）：22pt 文字，首选灰底、字重 500，恋爱/日常选了人时首选再用灰绿字；
// 大模型给的用次要灰。样式全由 CandidateStyle 定，两边不能各写各的。键盘一律系统字体，不用 AppFont。

import UIKit

final class CandidateCellView: UICollectionViewCell {
    static let identifier = "candidate"

    /// 一格的高度；候选栏与展开面板都按它算上下留白。
    static let height: CGFloat = 40

    private let label = UILabel()

    override init(frame: CGRect) {
        super.init(frame: frame)
        label.font = .systemFont(ofSize: 22)
        label.translatesAutoresizingMaskIntoConstraints = false
        contentView.addSubview(label)
        contentView.layer.cornerRadius = 8
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: contentView.leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(equalTo: contentView.trailingAnchor, constant: -10),
            label.topAnchor.constraint(equalTo: contentView.topAnchor),
            label.bottomAnchor.constraint(equalTo: contentView.bottomAnchor),
            contentView.heightAnchor.constraint(equalToConstant: Self.height),
            contentView.widthAnchor.constraint(greaterThanOrEqualToConstant: 40),
        ])
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func show(_ item: CandidateItem, highlighted: Bool, accent: Bool = false) {
        label.text = item.text
        label.textColor = UIColor(
            CandidateStyle.role(highlighted: highlighted, accent: accent, cloud: item.cloud).color)
        label.font = .systemFont(
            ofSize: 22, weight: CandidateStyle.weight(highlighted: highlighted))
        contentView.backgroundColor = CandidateStyle.background(highlighted: highlighted)
            .map(UIColor.init)
    }
}

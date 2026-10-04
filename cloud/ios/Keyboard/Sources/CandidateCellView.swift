// 面板里的一个候选：22pt 文字，首选灰底，大模型给的用强调色；宽度随文字。

import UIKit

final class CandidateCellView: UICollectionViewCell {
    static let identifier = "candidate"

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
            contentView.heightAnchor.constraint(equalToConstant: 40),
            contentView.widthAnchor.constraint(greaterThanOrEqualToConstant: 40),
        ])
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func show(_ item: CandidateItem, highlighted: Bool) {
        label.text = item.text
        label.textColor = item.cloud ? .tintColor : .label
        contentView.backgroundColor = highlighted ? UIColor(KeyStyle.highlightFill) : .clear
    }
}

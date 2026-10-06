// 点 ⌄ 展开的全部候选：UIKit 的 UICollectionView，按候选长短左对齐流式换行、纵向滚动。
// 不用 SwiftUI：它的 ScrollView 在键盘扩展里收不到滑动（候选栏那条横向的也一样），面板会「卡死」。

import UIKit

final class CandidatePanelView: UIView {
    var candidates: [CandidateItem] = [] {
        didSet {
            // 收起时不给候选：藏着的面板也参与布局，给了就每键重排（见 KeyboardViewController.applyPanel）
            assert(!isHidden || candidates.isEmpty, "收起的候选面板不该拿到候选")
            guard candidates != oldValue else { return }
            collection.reloadData()
            collection.setContentOffset(.zero, animated: false)
        }
    }

    /// 点了第几个候选。
    var onSelect: ((Int) -> Void)?

    /// 首选用强调色（与候选栏同一个来源：`KeyboardModel.accentFirstCandidate`）。
    var accentFirst = false {
        didSet {
            guard accentFirst != oldValue else { return }
            collection.reloadData()
        }
    }

    private let collection: UICollectionView

    override init(frame: CGRect) {
        let layout = LeftAlignedFlowLayout()
        layout.estimatedItemSize = UICollectionViewFlowLayout.automaticSize
        layout.minimumInteritemSpacing = 4
        layout.minimumLineSpacing = 6
        layout.sectionInset = UIEdgeInsets(
            top: 6, left: KeyStyle.sideMargin, bottom: 6, right: KeyStyle.sideMargin)
        collection = UICollectionView(frame: .zero, collectionViewLayout: layout)
        super.init(frame: frame)
        collection.backgroundColor = .clear
        collection.register(CandidateCellView.self, forCellWithReuseIdentifier: CandidateCellView.identifier)
        collection.dataSource = self
        collection.delegate = self
        collection.delaysContentTouches = false
        collection.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        collection.frame = bounds
        addSubview(collection)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

extension CandidatePanelView: UICollectionViewDataSource, UICollectionViewDelegate {
    func collectionView(_ collectionView: UICollectionView, numberOfItemsInSection section: Int) -> Int {
        candidates.count
    }

    func collectionView(_ collectionView: UICollectionView, cellForItemAt indexPath: IndexPath) -> UICollectionViewCell {
        let cell = collectionView.dequeueReusableCell(
            withReuseIdentifier: CandidateCellView.identifier, for: indexPath)
        if let cell = cell as? CandidateCellView {
            cell.show(
                candidates[indexPath.item], highlighted: indexPath.item == 0,
                accent: indexPath.item == 0 && accentFirst)
        }
        return cell
    }

    func collectionView(_ collectionView: UICollectionView, didSelectItemAt indexPath: IndexPath) {
        onSelect?(indexPath.item)
    }
}

/// 自动尺寸的流式布局缺省会把一行里的格子拉开对齐两端，候选要像文字一样左对齐。
final class LeftAlignedFlowLayout: UICollectionViewFlowLayout {
    override func layoutAttributesForElements(in rect: CGRect) -> [UICollectionViewLayoutAttributes]? {
        guard let attributes = super.layoutAttributesForElements(in: rect) else { return nil }
        var x = sectionInset.left
        var rowY: CGFloat = -1
        return attributes.map { original in
            guard original.representedElementCategory == .cell,
                  let item = original.copy() as? UICollectionViewLayoutAttributes
            else { return original }
            if item.frame.minY != rowY {
                rowY = item.frame.minY
                x = sectionInset.left
            }
            item.frame.origin.x = x
            x += item.frame.width + minimumInteritemSpacing
            return item
        }
    }
}

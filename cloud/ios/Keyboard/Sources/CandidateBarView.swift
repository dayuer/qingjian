// 候选栏里横向滚动的那一行：UIKit 的 UICollectionView。
// 不用 SwiftUI 的 ScrollView —— 它在键盘扩展里收不到滑动（展开的面板同理，见 CandidatePanelView）。
// 每格的样式由 CandidateStyle 定，与展开面板共用。拼音变了滚回开头；云端词插进来时不跳回开头。
// 格宽自己算（CandidateWidth，按词缓存），不走自适应测量；打字时只交首屏加余量的那些给集合视图，每键不必排全部 120 个，
// 手指一拖就交全部（宽度已缓存，排一次很便宜），甩动的落点按完整内容算，不会在中途的边界停住。

import UIKit

final class CandidateBarView: UIView {
    /// 点了第几个候选。
    var onSelect: ((Int) -> Void)?

    private var candidates: [CandidateItem] = []

    /// 交给集合视图的前几个候选数。
    private var shownCount = 0

    /// 打字时交的个数：一格至少 40pt，24 格够两屏多。
    private static let firstScreens = 24

    private let widths = CandidateWidthCache()

    /// 首选用强调色（`KeyboardModel.accentFirstCandidate`：恋爱、日常选了人）。
    private var accentFirst = false

    private let collection: UICollectionView

    override init(frame: CGRect) {
        let layout = UICollectionViewFlowLayout()
        layout.scrollDirection = .horizontal
        // 横向滚动时同一行里格与格的间距是 lineSpacing（minimumInteritemSpacing 管换行的行距）
        layout.minimumLineSpacing = 2
        layout.minimumInteritemSpacing = 2
        let vertical = (KeyStyle.candidateBarHeight - CandidateCellView.height) / 2
        layout.sectionInset = UIEdgeInsets(top: vertical, left: 4, bottom: vertical, right: 4)
        collection = UICollectionView(frame: .zero, collectionViewLayout: layout)
        super.init(frame: frame)
        collection.backgroundColor = .clear
        collection.showsHorizontalScrollIndicator = false
        collection.alwaysBounceHorizontal = true
        // 默认会先等一等看是不是要滑动，点上会有滞后；键盘上要即时出字
        collection.delaysContentTouches = false
        collection.register(
            CandidateCellView.self, forCellWithReuseIdentifier: CandidateCellView.identifier)
        collection.dataSource = self
        collection.delegate = self
        collection.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        collection.frame = bounds
        addSubview(collection)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// `fromStart`：拼音变了，滚回开头。只是插进了云端词（插在首选之后）时，用户若已经滑开，
    /// 让最左边看得见的那个候选留在原位，免得整行在手指底下错开一格、点到旁边那个。
    func show(_ candidates: [CandidateItem], accentFirst: Bool, fromStart: Bool) {
        guard candidates != self.candidates || accentFirst != self.accentFirst || fromStart else {
            return
        }
        let anchor = fromStart ? nil : visibleAnchor()
        // 云端词插进来时留住已经交上的那些，锚住的候选才还在集合视图里
        let kept = fromStart ? 0 : shownCount
        self.candidates = candidates
        shownCount = min(candidates.count, max(Self.firstScreens, kept))
        self.accentFirst = accentFirst
        collection.reloadData()
        guard !fromStart else {
            collection.setContentOffset(.zero, animated: false)
            return
        }
        guard let anchor, let index = candidates.firstIndex(of: anchor.item) else { return }
        collection.layoutIfNeeded()
        guard
            let frame = collection.layoutAttributesForItem(at: IndexPath(item: index, section: 0))?.frame
        else { return }
        let maxOffset = max(collection.contentSize.width - collection.bounds.width, 0)
        let x = min(max(frame.minX - anchor.inset, 0), maxOffset)
        collection.setContentOffset(CGPoint(x: x, y: 0), animated: false)
    }

    /// 滑开了时最左边看得见的候选与它离可视区左边的距离；还在开头时为 nil（插进来的词正常往后推就行）。
    private func visibleAnchor() -> (item: CandidateItem, inset: CGFloat)? {
        let offset = collection.contentOffset.x
        guard offset > 0 else { return nil }
        let first = collection.indexPathsForVisibleItems
            .compactMap { path in
                collection.layoutAttributesForItem(at: path).map { (path, $0.frame) }
            }
            .filter { $0.1.maxX > offset }
            .min { $0.1.minX < $1.1.minX }
        guard let (path, frame) = first, path.item < candidates.count else { return nil }
        return (candidates[path.item], frame.minX - offset)
    }

    private var scale: CGFloat {
        traitCollection.displayScale > 0 ? traitCollection.displayScale : UIScreen.main.scale
    }
}

extension CandidateBarView: UICollectionViewDataSource, UICollectionViewDelegateFlowLayout {
    func collectionView(
        _ collectionView: UICollectionView, numberOfItemsInSection section: Int
    ) -> Int {
        shownCount
    }

    func collectionView(
        _ collectionView: UICollectionView, layout collectionViewLayout: UICollectionViewLayout,
        sizeForItemAt indexPath: IndexPath
    ) -> CGSize {
        let inset = (collectionViewLayout as? UICollectionViewFlowLayout)?.sectionInset ?? .zero
        let limit = collectionView.bounds.width - inset.left - inset.right
        let width = widths.width(
            candidates[indexPath.item].text, highlighted: indexPath.item == 0, scale: scale, limit: limit)
        return CGSize(width: width, height: CandidateCellView.height)
    }

    func collectionView(
        _ collectionView: UICollectionView, cellForItemAt indexPath: IndexPath
    ) -> UICollectionViewCell {
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

    /// 手指一拖就把余下的全交上。
    func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
        guard shownCount < candidates.count else { return }
        let start = shownCount
        shownCount = candidates.count
        let paths = (start..<shownCount).map { IndexPath(item: $0, section: 0) }
        UIView.performWithoutAnimation { collection.insertItems(at: paths) }
    }
}

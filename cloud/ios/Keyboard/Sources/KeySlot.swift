// 键区里的一格：响应触摸的范围（cell）与在格子里画键帽要缩进多少（insets）。

import SwiftUI

struct KeySlot {
    let key: Key

    /// 在键区坐标里的触摸范围：相邻格子以缝隙中线为界，最外面的一直到键区边缘，键区里没有死区。
    let cell: CGRect

    /// 键帽相对格子的缩进：可见的键帽保持原来的大小与位置。
    let insets: EdgeInsets
}

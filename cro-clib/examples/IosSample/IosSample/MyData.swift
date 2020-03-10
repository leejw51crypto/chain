//
//  MyData.swift
//  IosSample
//
//  Created by leejw51 on 10/3/2020.
//  Copyright © 2020 leejw51. All rights reserved.
//

import Foundation
class MyData : Codable {
    var tendermint: String? = "ws://localhost:26657/websocket"
    var name: String? = "a"
    var passphras: String? = "b"
    var enckey: String? = "c"
    var mnemonics: String? = "d"
    
    
}

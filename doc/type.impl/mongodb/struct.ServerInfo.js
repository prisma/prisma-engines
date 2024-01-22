(function() {var type_impls = {
"mongodb":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-ServerInfo%3C'a%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#31-113\">source</a><a href=\"#impl-ServerInfo%3C'a%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a&gt; <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.address\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#56-58\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.address\" class=\"fn\">address</a>(&amp;self) -&gt; &amp;<a class=\"enum\" href=\"mongodb/options/enum.ServerAddress.html\" title=\"enum mongodb::options::ServerAddress\">ServerAddress</a></h4></section></summary><div class=\"docblock\"><p>Gets the address of the server.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.average_round_trip_time\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#65-67\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.average_round_trip_time\" class=\"fn\">average_round_trip_time</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/time/struct.Duration.html\" title=\"struct core::time::Duration\">Duration</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the weighted average of the time it has taken for a server check to round-trip\nfrom the driver to the server.</p>\n<p>This is the value that the driver uses internally to determine the latency window as part of\nserver selection.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.last_update_time\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#71-73\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.last_update_time\" class=\"fn\">last_update_time</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"bson/datetime/struct.DateTime.html\" title=\"struct bson::datetime::DateTime\">DateTime</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the last time that the driver’s monitoring thread for the server updated the internal\ninformation about the server.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.max_wire_version\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#76-78\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.max_wire_version\" class=\"fn\">max_wire_version</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.i32.html\">i32</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the maximum wire version that the server supports.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.min_wire_version\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#81-83\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.min_wire_version\" class=\"fn\">min_wire_version</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.i32.html\">i32</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the minimum wire version that the server supports.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.replica_set_name\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#86-88\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.replica_set_name\" class=\"fn\">replica_set_name</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.str.html\">str</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the name of the replica set that the server is part of.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.replica_set_version\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#91-93\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.replica_set_version\" class=\"fn\">replica_set_version</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.i32.html\">i32</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the version of the replica set that the server is part of.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.server_type\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#96-98\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.server_type\" class=\"fn\">server_type</a>(&amp;self) -&gt; <a class=\"enum\" href=\"mongodb/enum.ServerType.html\" title=\"enum mongodb::ServerType\">ServerType</a></h4></section></summary><div class=\"docblock\"><p>Get the type of the server.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.tags\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#101-103\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.tags\" class=\"fn\">tags</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&amp;<a class=\"type\" href=\"mongodb/options/type.TagSet.html\" title=\"type mongodb::options::TagSet\">TagSet</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the tags associated with the server.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.error\" class=\"method\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#110-112\">source</a><h4 class=\"code-header\">pub fn <a href=\"mongodb/struct.ServerInfo.html#tymethod.error\" class=\"fn\">error</a>(&amp;self) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&amp;<a class=\"struct\" href=\"mongodb/error/struct.Error.html\" title=\"struct mongodb::error::Error\">Error</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Gets the error that caused the server’s state to be transitioned to Unknown, if any.</p>\n<p>When the driver encounters certain errors during operation execution or server monitoring,\nit transitions the affected server’s state to Unknown, rendering the server unusable for\nfuture operations until it is confirmed to be in healthy state again.</p>\n</div></details></div></details>",0,"mongodb::event::sdam::ServerDescription"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-ServerInfo%3C'a%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#22-29\">source</a><a href=\"#impl-Serialize-for-ServerInfo%3C'a%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a&gt; <a class=\"trait\" href=\"serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#23-28\">source</a><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"serde/ser/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;S&gt;(&amp;self, serializer: S) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;S::<a class=\"associatedtype\" href=\"serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, S::<a class=\"associatedtype\" href=\"serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<span class=\"where fmt-newline\">where\n    S: <a class=\"trait\" href=\"serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</span></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"serde/ser/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","mongodb::event::sdam::ServerDescription"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-ServerInfo%3C'a%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#115-142\">source</a><a href=\"#impl-Debug-for-ServerInfo%3C'a%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#116-141\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","mongodb::event::sdam::ServerDescription"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-ServerInfo%3C'a%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#17\">source</a><a href=\"#impl-Clone-for-ServerInfo%3C'a%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#17\">source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","mongodb::event::sdam::ServerDescription"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Display-for-ServerInfo%3C'a%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#144-194\">source</a><a href=\"#impl-Display-for-ServerInfo%3C'a%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a> for <a class=\"struct\" href=\"mongodb/struct.ServerInfo.html\" title=\"struct mongodb::ServerInfo\">ServerInfo</a>&lt;'a&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/mongodb/sdam/public.rs.html#145-193\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Display.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/struct.Error.html\" title=\"struct core::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Display.html#tymethod.fmt\">Read more</a></div></details></div></details>","Display","mongodb::event::sdam::ServerDescription"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()
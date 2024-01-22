(function() {var type_impls = {
"tonic":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-InterceptorLayer%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#83\">source</a><a href=\"#impl-Clone-for-InterceptorLayer%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptorLayer.html\" title=\"struct tonic::service::interceptor::InterceptorLayer\">InterceptorLayer</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#83\">source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptorLayer.html\" title=\"struct tonic::service::interceptor::InterceptorLayer\">InterceptorLayer</a>&lt;F&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","tonic::service::interceptor::InterceptorFn"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Layer%3CS%3E-for-InterceptorLayer%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#88-97\">source</a><a href=\"#impl-Layer%3CS%3E-for-InterceptorLayer%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;S, F&gt; <a class=\"trait\" href=\"tower_layer/trait.Layer.html\" title=\"trait tower_layer::Layer\">Layer</a>&lt;S&gt; for <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptorLayer.html\" title=\"struct tonic::service::interceptor::InterceptorLayer\">InterceptorLayer</a>&lt;F&gt;<span class=\"where fmt-newline\">where\n    F: <a class=\"trait\" href=\"tonic/service/trait.Interceptor.html\" title=\"trait tonic::service::Interceptor\">Interceptor</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>,</span></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Service\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Service\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"tower_layer/trait.Layer.html#associatedtype.Service\" class=\"associatedtype\">Service</a> = <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptedService.html\" title=\"struct tonic::service::interceptor::InterceptedService\">InterceptedService</a>&lt;S, F&gt;</h4></section></summary><div class='docblock'>The wrapped service</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.layer\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#94-96\">source</a><a href=\"#method.layer\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"tower_layer/trait.Layer.html#tymethod.layer\" class=\"fn\">layer</a>(&amp;self, service: S) -&gt; Self::<a class=\"associatedtype\" href=\"tower_layer/trait.Layer.html#associatedtype.Service\" title=\"type tower_layer::Layer::Service\">Service</a></h4></section></summary><div class='docblock'>Wrap the given service with the middleware, returning a new service\nthat has been decorated with the middleware.</div></details></div></details>","Layer<S>","tonic::service::interceptor::InterceptorFn"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-InterceptorLayer%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#83\">source</a><a href=\"#impl-Debug-for-InterceptorLayer%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptorLayer.html\" title=\"struct tonic::service::interceptor::InterceptorLayer\">InterceptorLayer</a>&lt;F&gt;</h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#83\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/struct.Formatter.html\" title=\"struct core::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"type\" href=\"https://doc.rust-lang.org/1.75.0/core/fmt/type.Result.html\" title=\"type core::fmt::Result\">Result</a></h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"https://doc.rust-lang.org/1.75.0/core/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","tonic::service::interceptor::InterceptorFn"],["<section id=\"impl-Copy-for-InterceptorLayer%3CF%3E\" class=\"impl\"><a class=\"src rightside\" href=\"src/tonic/service/interceptor.rs.html#83\">source</a><a href=\"#impl-Copy-for-InterceptorLayer%3CF%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for <a class=\"struct\" href=\"tonic/service/interceptor/struct.InterceptorLayer.html\" title=\"struct tonic::service::interceptor::InterceptorLayer\">InterceptorLayer</a>&lt;F&gt;</h3></section>","Copy","tonic::service::interceptor::InterceptorFn"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()
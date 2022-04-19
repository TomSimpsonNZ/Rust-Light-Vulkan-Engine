# Rust Light Vulkan Engine

This is a translation of Brendan Galea's Vulkan tutorial into rust using the [Ash](https://github.com/MaikKlein/ash) crate.

Original tutorial: [Brendan Galea's YouTube Page](https://www.youtube.com/c/BrendanGalea)

Each commit will correspond to a video from this tutorial.

## Requirements

You will need to have the [LunarG Vulkan SDK](https://www.lunarg.com/vulkan-sdk/) installed and on linux 
you will need to add the library and layers to your path.

## Things to Note

- Unlike the tutorial, this translation will use the winit window API as it is fully written in rust. 
This will result in some large deviations from the videos.
- Due to the methodology of rust being incredibly different to that of C++, the structure of the code 
will be different in some areas. These changes will be highlighted with a hopefully correct :) explanation 
of why this is the case.
- To get the most out of the tutorial, I recommend trying to follow along with the video and translate
the code yourself and use this repository if you get stuck or want to copy large amounts of code (such 
as videos 3, 4, and 5). 

To use the logging functionality in this code, you need to set the ```RUST_LOG``` environment variable:

#### Windows PowerShell
```
$env:RUST_LOG="debug"
```

#### Linux
```
export RUST_LOG="debug"
```

More information about using this crate can be found in the [documentation](https://docs.rs/log/0.4.14/log/).

## Acknowledgements

Big thanks to [Brendan Galea](https://www.youtube.com/c/BrendanGalea) for making the tutorial that this code is based on, and Alexander Overvoorde
for making [The Vulkan Tutorial](https://vulkan-tutorial.com/) that I first learnt Vulkan from. Also thanks to [Adrien Ben](https://github.com/adrien-ben)
for translating Alex's tutorial, a bunch of this code was yoinked from [that repo](https://github.com/adrien-ben/vulkan-tutorial-rs) :)

# 1: Opening a Window ([link](https://www.youtube.com/watch?v=lr93-_cC8v4&ab_channel=BrendanGalea))
- Later versions of [winit](https://docs.rs/winit/0.25.0/winit/) (0.20+) use an architecture that is very different to glfw. Because of this the structure presented in the tutorial will not work, or will be very weird to implement. As such, there is no lve_window struct and ownership of the window has been moved to the application (first_app).

# 2: Graphics Pipeline Overview ([link](https://www.youtube.com/watch?v=_riranMmtvI&ab_channel=BrendanGalea))
- A `build.rs` file was added to compile the shader files when the program is built. This is just a replacement 
for the shell script presented in the video.
- "We're about half way to seeing our first triangle on screen"... I've never heard a more blatant lie.

# 3: Device Setup & Pipeline cont. ([link](https://www.youtube.com/watch?v=LYKlEIzGmW4&ab_channel=BrendanGalea))
- Due to the nature of Rust, a lot of the functions that were `void` in the tutorial now return things. This is so we can properly initialise the `LveDevice` struct by allowing the functions to borrow these Vulkan structs.
- `device_extensions` does is not a global constant anymore as ash requires the extension names to be given by a functions, and hence can't be stored in a constant. It can now be found in the `LveDevice::get_device_extensions()` function.
- Due to the lack of lve_window module, it was more convenient to use the `create_surface()` function from the [ash-window](https://docs.rs/ash-window/0.7.0/ash_window/) crate in the `LveDevice::create_surface()` function.
The naming here is a little confusing, sorry about that.
- You will also note that ash has two surface types, `Surface` and `vk::SurfaceKHR`. The former is a struct containing the functions that act on the surface, while the latter is the Vulkan surface itself.
- The biggest deviation in this code is the fact that the `LveDevice` now owns the `LvePipeline` instead of the application owning both. In the tutorial, Brendan says that having a reference to the device in the pipeline 
can be unsafe as the device needs to be destroyed after the pipeline. In Rust, if it can be unsafe it won't compile, so some restructuring had to be done. 

![new structure](./images/new_structure.png)

- Overall, the way that extensions are handled in this implementation are slightly different so don't expect the exact same console output as the one shown in the video.

# 4: Fixed Function Pipeline Stages ([link](https://www.youtube.com/watch?v=ecMcXW6MSYU&ab_channel=BrendanGalea))
- Some pipeline builder functions were commented out as they were optional and required null pointers, something I'm not sure is implemented in rust.
- Pipeline destructor function was already implemented in the previous commit.

# 5.1: Swap Chain Overview ([link](https://www.youtube.com/watch?v=IUYH74MqxOA&ab_channel=BrendanGalea))
- Architecture change made in tutorial 3 commit was once again changed, as an approach more similar to that given by the tutorial was found. The `lve_device`, `lve_swapchain`, and `lve_pipeline` are all now owned by the application.
The conflicting issue in tutorial 3 was the destruction of the Vulkan objects, which most of the time require a reference to the device. By making the application handle the destruction of everything when the struct is dropped, we can pass the device to the other modules without having any cyclical linking trees.
    - Each struct now has a ` pub unsafe fn destroy()` function that handles the destruction of its fields.
- public get functions are discouraged in Rust, so instead those variables were passed to functions when they were needed and will individually be made public when they are needed in other modules.
- No vectors need to be resized in rust, so all those lines were skipped.
- All functions that were `void` in the tutorial now return their respective structs similarly to tutorial 3.
- Some reformatting was done (witch should have been done during the other commits). This was done using rusts inbuilt formatter, so should not be too hard to replicate
- My machine is currently giving validation errors at the end of this tutorial. For now I will leave this issue to see if fully implementing everything in the next tutorial fixes the issue, if not then there will be another commit with a fix (hopefully)

# 5.2: Command Buffers Overview ([link](https://www.youtube.com/watch?v=_VOR6q3edig&ab_channel=BrendanGalea))
- The validation errors from the previous tutorial have not disappeared. Will look into it more.
- The triangle also does not display color correctly, being a dark maroon when it's supposed to be red, 
and bright green when the other color values are set to 1.0 in the fragment shader file. This is probably linked to the validation errors.

## 5.2.1: Debugging
- From reading the validation layer output (should have done this when they came up :) ), it became clear that the depth stencil create info struct was not given a format, hence causing the first half of the errors.
- For the remaining errors, something much stranger was happening. The errors were along the lines of: ` pCreateInfos[0].pColorBlendState->pAttachments[0].srcColorBlendFactor (51) does not fall within the begin..end range of the core VkBlendFactor enumeration tokens and is not an extension added token.` Considering that this value (`srcColorBlendFactor` in this case) was set to be an ash enum (such as `vk::BlendFactor::ONE` which should have a vale of 1), it is hard to believe that `vkCreateGraphicsPipelines()` is receiving a value of 51 in this example. This seems to be an issue with the `color_blend_attachment` and `color_blend_info` structs as they seem 
to be avoiding rusts memory safety checks. As a result, passing this struct around resulted in random bits of 
memory being read which caused strange errors. 
    - This also caused the program to behave differently every time it was run.
- To solve this, the `color_blend_attachment` and `color_blend_info` struct definitions were moved to the `LvePipeline::create_graphics_pipeline()` function so that they would never leave scope.
- This seems to be an issue with `ash`, might be solved in more recent versions. 

# 6: Vertex Buffers ([link](https://www.youtube.com/watch?v=mnKp501RXDc&ab_channel=BrendanGalea))
- No changes of note from the tutorial
- A solution to the exercise in the video can be found in the fork.

# 7: Fragment Interpolation ([link](https://www.youtube.com/watch?v=ngoZZkMuCOM&ab_channel=BrendanGalea))
- They is no rust equivalent of `offsetof()` that I am aware of at this point in time. So a workaround was used that is only slightly better than just hard coding in an offset of 8 bytes :).
- I also decided to finally remove the drop implementation for the `lve_*` sub-modules. They weren't really doing anything.

# 8: Swapchain Recreation and Dynamic Viewports ([link](https://www.youtube.com/watch?v=0IIqvi3Z0ng&ab_channel=BrendanGalea))
- Winit is very different to glfw, so the first part of this tutorial is very different (but in my opinion, a lot nicer).
- Because of winit's differences, I saw a, in my opinion, much nicer way of handling window resizing. Whenever winit detects that the window is resized, the `recreate_swapchain()` function is called. Since we know that the 
old swapchain is out of date at this point. Then in the `draw()` function, recreate swapchain is only called if there is some other event that causes the swapchain to become out of date.
- There is a possible mistake in Brendan's code which he doesn't notice as he is on macOS. As he points out macOS pauses the program until the window has finished resizing, meaning that he is not creating as many new swapchains as other platforms such as windows or linux. Since the tutorial code does not destroy the old swapchain when it creates a new one, Vulkan begins to have a cry after a large amount of resizing. To resolve this a call to the `LveSwapchain::destroy()` function was added to `recreate_swapchain()`.
- This is also happening to the pipeline, so a call to `LvePipeline::destroy()` was also added.
- The `viewport_count()` and `scissor_count()` needed to be set to 1 while creating `viewport_info` as Vulkan thinks that 0 > 1 ...
- Implementation of old swapchain is also a bit different, instead of passing the whole `LveSwapchain` struct, we just pass the old swapchain_khr into `LveSwapchain::new()`. This is wrapped in an option to account for the times where there is no old swapchain.

# 9: Push Constants ([link](https://www.youtube.com/watch?v=wlLGLWI9Fdc&ab_channel=BrendanGalea))
- I could not find a nice way to align the fields of the `SimplePushConstantData` struct, so I just made the position vector a `vec4`. I am not proud of this :).
    - If I find a better method of alignment in the future I will come back and fix this.

## Push Constant Fix:
- It seems that Ash's implementation of push constants requires that the push constants ranges for the 
vertex and the fragment shaders be split into separate structs.
- This also applies for when actually pushing these values, one push for the offset and one push for the color.
- Finally learnt about type aliases, so made some changes that should hopefully lead to less errors in the future.
- Also removed the need for the bytemuck crate as it seemed unnecessary for what I was using it for, wrote some functions to get the specific push constants in slice form.

## Push Constant Fix (for real this time):
- After moving onto the next tutorial, it became apparent that the previous fix was not going to work. It was time to figure out how to properly align the fields of a struct. Since using `#[repr(align(16))]` on a struct 
only aligns the whole struct and not it's fields, I had to get a bit creative. By defining the wrapper struct `Align16<t>(pub T)`, each of the fields of the `SimplePushConstantData` struct can now be aligned.
- Push constants were also made to only affect the vertex shader, as ash was making it hard to send push constants to both the vertex and fragment shaders.

# 10: 2D Transformations ([link](https://www.youtube.com/watch?v=gxUcgc88tD4&ab_channel=BrendanGalea))
- Had to do a little bit of refactoring to allow the rotation to be mutable in the `render_game_objects()` function.
- Also decided to make the type declaration for `Pos` and `Color` and so on module specific to avoid alignment weirdness, but this could become confusing.
- Will fork the cool animation.

# 11: Renderer & Systems ([link](https://www.youtube.com/watch?v=uGRSTRGlZVs&ab_channel=BrendanGalea))
- No big changes of note.
- Will fork the gravity simulation.

# Refactor
- In making the little gravity simulation after the last tutorial, it became apparent that the way the code was set up was not going to work. In the current state, each game object would need its own version of the model, even if it was the exact same model that another object was using. Since we are not modifying the models in the code, this is just inefficient and it takes a while to load and destroy all the models.
    - In the previous version of the code, having the game object contain a reference to the model was (I think, could be wrong) impossible as it was ambiguous when the model should be destroyed.
- To solve this issue, the `LveModel` and the `LveDevice` structs were made to return smart pointers from their constructors. This allows for many different game objects to all access the same model and then release the model (and it vertex buffers) from memory when no game object is using it anymore. To do this, a smart pointer to the `LveDevice` was also needed so that the model could de allocate when it was dropped. While I was at it, I implemented the drop trait for the rest of the modules in the engine, allowing for the same behaviour.
- I will merge these changes with the Gravity sim branch 

## Small edit
- Removed the ID field from the model, the whole point of the refactor was that there would only be one version of each model :)

# 12: Euler Angles & Homogeneous Coordinates ([Link](https://www.youtube.com/watch?v=0X_kRtyVzm4&ab_channel=BrendanGalea))
- No big changes of note.

# 13: Projection Matrices
- No big changes of note.
- Noticed a few bugs, program crashes when run in release mode. Once again there seems to be an issue with the ash builder patterns. Will look into this further.

# Bug Fixes
- While testing the code on a few devices I ran into a few issues, the next few commits are aimed at solving these.

## Linux Issues
- By copying some of the Readme from [this repo](https://github.com/adrien-ben/vulkan-tutorial-rs), I forgot to update the 
command for enabling the logging functionality on linux. Sorry for anyone that got confused by that...
- Updated `build.rs` to point to the correct directory and correct file name, as there is no such thing as a `.exe` in linux.
- Linux uses a different window manager than windows, so the extensions need to be different.

## Dangling Pointers
- Dangling pointers have been an issue with this code for a while now and I've just sort of been ignoring them. However, now that I know what is causing them 
I can implement a proper fix.
- In Ash, when `.build()` is used on a builder, the lifetime information of the builder is lost, leading to dangling pointers. Because of this, `.build()` should be avoided where possible and
the `Deref` implementation for the builders should be used. 
    - `.build()` must be used if multiple structs are combined into a slice, as is the case in a few places. 
    - For the cases where the vulkan functions require a slice of one element, then `std::slice::from_ref()` can be used to maintain the lifetime information of the struct.
    More info on this can be found on the [ash crates.io page](https://crates.io/crates/ash/0.33.3+1.2.191).
- The `default_pipline_config_info()` function is a bit tricky, since these builders are being passed into a struct, we have to use `build()`, otherwise the values will go out of scope
leading to dangling references. For now I have used `Rc<T>` to solve this for the problematic builders, but a better solution should be found.

## General
- Minimizing the window would crash the program due to the check for the renderer being located in the `begin_frame()` function. 
- Not a bug fix, but decided to upgrade to the latest version of ash. This doesn't seem to have any effect.
- Removed most warnings as they were getting on my nerves :)

# 14: Camera (View) Transform ([Link](https://www.youtube.com/watch?v=rvJHkYnAR3w&ab_channel=BrendanGalea))
- Decided to make `LveCamera` follow the builder pattern. There's probably no advantage to this, just felt like trying it out :)

# 15: Game loops & User input ([Link](https://www.youtube.com/watch?v=rvJHkYnAR3w&ab_channel=BrendanGalea))
- User input was not made very easy by winit, so I had to make a little work around to ensure that multiple keys can be pressed at the same time.
- Timing was very tricky to get right as there are a few things with winit that I think were not made very clear. Firstly, the `MainEventsCleared` flag 
is not occur as often as I would expect, so the frames were only drawn to the screen occasionally instead of (almost) every iteration of the events loop.
This caused the camera to jump around seemingly randomly when moving and looking around. This also made it seem like the FIFO present mode was not working
as the fps was well above the refresh rate of my monitor.
- Moving the `run()` call to the empty section of the match statement fixed this issue, however it introduced many more issues, such as not being able to resize the 
window and having large amounts of input lag for a short period after application startup in FIFO.
- All of these issues are stemming from the way that winit handles window events and the main loop. The more I get into it the more I dislike it. After doing a bit of 
research it seems that people are recommending SDL2 as a window manager. This will take a while to refactor though, so I'll get round to it when I have the time.

# Bug Fixes
Finally got some time to work on this again!
- The more observant of you might have noticed the reason for all my errors in the previous commit. I was measuring the frame time from after the frame was rendered and presented to the window. This meant that in the FIFO present mode, the render call would block to ensure that the frame rate was 144Hz in my case. So each frame was actually taking significantly longer than what the timer would indicate, meaning the camera movement was significantly slowed down. Moving the refresh of `current_time` to before the draw call fixed all the issues, no need to change windowing API!

=> I am a bit silly :)

- Made a better fps counter and made it print to the window title to clear up the log.
- Reformatted all the files to make them look nice and pretty.

# 16: Index and Staging Buffers ([Link](https://www.youtube.com/watch?v=qxuvQVtehII))
- Renamed the `Builder` struct to `ModelData` and `create_index_buffers()` to `create_index_buffer()` as per Brendan's recomendations in the comments
- Made the indices field of `ModelData` an option to be more in line with Rust's syntax

# 17: Loading 3D Models ([Link](https://www.youtube.com/watch?v=jdiPVfIHmEA))
- Used the [tobj](https://docs.rs/tobj/latest/tobj/) crate to do the object loading. Is supposedly based off tinyobjectloader, but there are a few differences.
- To be able to hash the `Vertex` type, I needed to implement the `Hash` trait for `f32`. This is not possible in rust by default, so used the [oredered-float](https://docs.rs/ordered-float/latest/ordered_float/index.html) crate to work around this. Defined a new type `Hf32` to save typing
- Rust already handles the combination of hash values using `std::hash::Hasher`, so the `hash_combine()` function is useless, all we need to do is implement the `Hash` trait for the `Vertex` type, then call `Vertex::hash(&mut hasher)` when we need it.
- No such thing as operator overloading in Rust, instead you implement traits. The equivalent to overloading the `==` operator is to implement the trait `PartialEq` for the type. 
- I don't think you can implement a trait to give the same effect as overloading the `()` operator like Brendan does in the video, so instead we will just call `vertex.hash()`
- Since we build the vertices Vec after the iterator, we cannot use the same method for storing the index in the `HashMap` as the video. Instead we will just make a counter that will increment every time there is a new unique vertex. This should also be faster as well :)

# 18: Diffuse Shading ([Link](https://www.youtube.com/watch?v=wfh2N4u-nOU))
- We don't have the same issue as Brendan in regards to the color vector, as it is either filled with values or empty. But his fix did inspire a small optimisation.

# 19.1: Uniform Buffers ([Link](https://www.youtube.com/watch?v=may_GMkfs5k))
- Didn't make any getter functions as it is bad practice in rust, instead just made the fields public.
- Changed the buffers to be options in the model to make creating a model with no mesh nicer. Also allows for built in checking of wether or not the model has an index buffer.
- The model type no longer needs to have a reference to a LveDevice to function, so I have removed that.
- Added an enum to define the type of buffer. This is only useful for validation layer messages so I can see what buffers are being deallocated when instead of jus "Dropping Buffer"
- Validation layers actually picked up the error in this video, but no fix in this commit :)
- It's getting late at the time of writing, so the `FrameInfo` struct will not be included in this commit, but in the next one where we fix the `NonCoherentAtomSize` bug!

# 19.2 Uniform Buffers Part 2
- Was declaring a new `global_ubo_buffer` on every frame, moved it's declaration to the `VulkanApp` constructor and then stored an `Rc<>` of it there.
- `FrameInfo` struct is now added.

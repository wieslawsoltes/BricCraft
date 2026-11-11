"""Optional native Ganesh benchmark backend; requires PyOpenGL and EGL.

The selected EGL driver is reported explicitly. A software driver such as
llvmpipe is useful for repeatable comparisons but is not browser/hardware FPS.
The context lives for the benchmark process; it is never used by the game.
"""
import ctypes
import os


class NativeGlBackend:
    def __init__(self, skia):
        os.environ.setdefault('PYOPENGL_PLATFORM', 'egl')
        os.environ.setdefault('EGL_PLATFORM', 'surfaceless')
        from OpenGL import EGL, GL
        self.skia, self.EGL, self.GL = skia, EGL, GL
        self.display = EGL.eglGetDisplay(EGL.EGL_DEFAULT_DISPLAY)
        major, minor = EGL.EGLint(), EGL.EGLint()
        if not EGL.eglInitialize(self.display, major, minor):
            raise RuntimeError('EGL initialization failed; use --backend raster on this system')
        attributes = (EGL.EGLint * 13)(
            EGL.EGL_SURFACE_TYPE, EGL.EGL_PBUFFER_BIT,
            EGL.EGL_RENDERABLE_TYPE, EGL.EGL_OPENGL_BIT,
            EGL.EGL_RED_SIZE, 8, EGL.EGL_GREEN_SIZE, 8,
            EGL.EGL_BLUE_SIZE, 8, EGL.EGL_ALPHA_SIZE, 8, EGL.EGL_NONE)
        config, count = EGL.EGLConfig(), EGL.EGLint()
        EGL.eglChooseConfig(self.display, attributes, ctypes.byref(config), 1, count)
        if count.value != 1:
            raise RuntimeError('No suitable EGL OpenGL pixel format')
        EGL.eglBindAPI(EGL.EGL_OPENGL_API)
        self.egl_context = EGL.eglCreateContext(self.display, config, EGL.EGL_NO_CONTEXT, None)
        self.pbuffer = EGL.eglCreatePbufferSurface(self.display, config,
            (EGL.EGLint * 5)(EGL.EGL_WIDTH, 1, EGL.EGL_HEIGHT, 1, EGL.EGL_NONE))
        EGL.eglMakeCurrent(self.display, self.pbuffer, self.pbuffer, self.egl_context)
        self.context_info = {
            'renderer': GL.glGetString(GL.GL_RENDERER).decode(),
            'vendor': GL.glGetString(GL.GL_VENDOR).decode(),
            'version': GL.glGetString(GL.GL_VERSION).decode(),
        }
        self.context = skia.GrDirectContext.MakeGL(skia.GrGLInterface.MakeEGL())
        if self.context is None:
            raise RuntimeError('Skia could not create a Ganesh EGL context')
        self.name = 'native Skia Ganesh OpenGL / ' + self.context_info['renderer']

    def surface(self, width, height):
        surface = self.skia.Surface.MakeRenderTarget(self.context, self.skia.Budgeted.kNo,
            self.skia.ImageInfo.MakeN32Premul(width, height))
        if surface is None:
            raise RuntimeError('Skia could not allocate the requested GPU surface')
        return surface

    def synchronize(self):
        self.context.flushAndSubmit(self.skia.GrSyncCpu.kYes)
        self.GL.glFinish()

/* <arc-3d> — toon-shaded 3D arc matching the 2D sticker:
   squircle head, cel (3-step) shading, black inverted-hull outlines,
   pink tentacle undersides. Idle bob + cursor tracking. PNG fallback. */
(function () {
  class arc3D extends HTMLElement {
    connectedCallback() {
      if (this._started) return;
      this._started = true;
      this.style.display = 'block';
      if (!this.style.width) this.style.width = '100%';
      if (!this.style.height) this.style.height = '100%';
      this._init();
    }

    disconnectedCallback() {
      if (this._raf) cancelAnimationFrame(this._raf);
      if (this._ro) this._ro.disconnect();
      if (this._onMove) window.removeEventListener('pointermove', this._onMove);
      if (this._onDown && this._renderer) this._renderer.domElement.removeEventListener('pointerdown', this._onDown);
      if (this._renderer) this._renderer.dispose();
    }

    _fallback() {
      this.innerHTML = '<img src="./arc.png" alt="arc" style="width:100%; height:100%; object-fit:contain; padding:80px 0;">';
    }

    async _init() {
      let THREE;
      try {
        THREE = await import('https://unpkg.com/three@0.160.0/build/three.module.js');
      } catch (e) { this._fallback(); return; }

      let renderer;
      try {
        renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
      } catch (e) { this._fallback(); return; }
      this._renderer = renderer;
      renderer.setClearColor(0x000000, 0);
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      renderer.domElement.style.cssText = 'width:100%; height:100%; display:block; cursor:pointer;';
      this.appendChild(renderer.domElement);

      // Poke interaction — click/tap spins + squishes arc (and splashes)
      let spinVel = 0, spinOff = 0, squashImp = 0, baseRot = 0, splashQueued = false, xOff = 0;
      this._onDown = () => {
        spinVel += 9;
        squashImp = 0.22;
        splashQueued = true;
        window.dispatchEvent(new CustomEvent('arc-poked'));
      };
      renderer.domElement.addEventListener('pointerdown', this._onDown);

      const scene = new THREE.Scene();
      const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 50);
      camera.position.set(0, 0.5, 6.4);
      camera.lookAt(0, -0.1, 0);

      // Cel lighting: one key light + strong ambient = flat sticker look
      scene.add(new THREE.AmbientLight(0xffffff, 1.15));
      const key = new THREE.DirectionalLight(0xffffff, 1.4);
      key.position.set(2.5, 4, 5);
      scene.add(key);

      // 3-step toon gradient
      const steps = new Uint8Array([135, 205, 255]);
      const gradientMap = new THREE.DataTexture(steps, steps.length, 1, THREE.RedFormat);
      gradientMap.minFilter = THREE.NearestFilter;
      gradientMap.magFilter = THREE.NearestFilter;
      gradientMap.needsUpdate = true;

      const INK = 0x17131f;
      const bodyMat = new THREE.MeshToonMaterial({ color: 0xa69af2, gradientMap });
      const pinkMat = new THREE.MeshToonMaterial({ color: 0xf7cfe0, gradientMap });
      const darkMat = new THREE.MeshBasicMaterial({ color: INK });
      const outlineMat = new THREE.MeshBasicMaterial({ color: INK, side: THREE.BackSide });

      const arc = new THREE.Group();
      scene.add(arc);

      // --- Head: squircle (sphere pushed toward a rounded cube) ---
      const squircle = (geo, e) => {
        const p = geo.attributes.position;
        for (let i = 0; i < p.count; i++) {
          const x = p.getX(i), y = p.getY(i), z = p.getZ(i);
          p.setXYZ(i,
            Math.sign(x) * Math.pow(Math.abs(x), e),
            Math.sign(y) * Math.pow(Math.abs(y), e),
            Math.sign(z) * Math.pow(Math.abs(z), e)
          );
        }
        geo.computeVertexNormals();
        return geo;
      };
      const headGeo = squircle(new THREE.SphereGeometry(1, 72, 56), 0.72);

      const headGrp = new THREE.Group();
      headGrp.position.y = 0.55;
      arc.add(headGrp);

      const head = new THREE.Mesh(headGeo, bodyMat);
      head.scale.set(1.18, 1.12, 0.95);
      headGrp.add(head);
      const headOutline = new THREE.Mesh(headGeo, outlineMat);
      headOutline.scale.set(1.18 * 1.045, 1.12 * 1.045, 0.95 * 1.045);
      headGrp.add(headOutline);

      // Eyes (flat black, like the sticker)
      for (const sx of [-1, 1]) {
        const eye = new THREE.Mesh(new THREE.SphereGeometry(0.11, 24, 24), darkMat);
        eye.position.set(sx * 0.42, 0.0, 0.9);
        headGrp.add(eye);
      }

      // Smile — lower half-torus, proud of the face
      const smile = new THREE.Mesh(new THREE.TorusGeometry(0.17, 0.05, 12, 32, Math.PI), darkMat);
      smile.position.set(0, -0.28, 0.95);
      smile.rotation.z = Math.PI;
      headGrp.add(smile);

      // --- Tentacles: lavender tube + pink underside + outline hull ---
      const tentacles = [];
      const curve = new THREE.CatmullRomCurve3([
        new THREE.Vector3(0.35, -0.45, 0),
        new THREE.Vector3(0.95, -0.85, 0),
        new THREE.Vector3(1.5, -1.15, 0),
        new THREE.Vector3(1.85, -1.0, 0)
      ]);
      const tubeGeo = new THREE.TubeGeometry(curve, 24, 0.15, 12);
      const tubeOutlineGeo = new THREE.TubeGeometry(curve, 24, 0.19, 12);
      const pinkCurve = new THREE.CatmullRomCurve3(
        curve.points.map(p => new THREE.Vector3(p.x, p.y - 0.055, p.z))
      );
      const pinkGeo = new THREE.TubeGeometry(pinkCurve, 24, 0.12, 12);
      const tipGeo = new THREE.SphereGeometry(0.15, 16, 16);
      const tipOutlineGeo = new THREE.SphereGeometry(0.19, 16, 16);

      for (let i = 0; i < 8; i++) {
        const angle = (i / 8) * Math.PI * 2 + Math.PI / 8;
        const grp = new THREE.Group();
        grp.rotation.y = angle;
        grp.add(new THREE.Mesh(tubeOutlineGeo, outlineMat));
        grp.add(new THREE.Mesh(pinkGeo, pinkMat));
        grp.add(new THREE.Mesh(tubeGeo, bodyMat));
        const tipOutline = new THREE.Mesh(tipOutlineGeo, outlineMat);
        tipOutline.position.set(1.85, -1.0, 0);
        grp.add(tipOutline);
        const tip = new THREE.Mesh(tipGeo, bodyMat);
        tip.position.set(1.85, -1.0, 0);
        grp.add(tip);
        arc.add(grp);
        tentacles.push({ grp, angle, phase: i * 0.9 });
      }

      // --- Water: toon surface + ripples + bubbles ---
      const WY = -0.35;
      const waterGeo = new THREE.PlaneGeometry(22, 12, 56, 34);
      waterGeo.rotateX(-Math.PI / 2);
      const waterBase = Float32Array.from(waterGeo.attributes.position.array);
      const waterMat = new THREE.MeshToonMaterial({ color: 0x7fb0e8, gradientMap, transparent: true, opacity: 0.62, depthWrite: false });
      const waterMesh = new THREE.Mesh(waterGeo, waterMat);
      waterMesh.position.y = WY;
      waterMesh.renderOrder = 2;
      scene.add(waterMesh);

      // deep-water backdrop — fills everything below the waterline
      const deepMat = new THREE.MeshBasicMaterial({ color: 0x74a5e4, transparent: true, opacity: 0.68, depthWrite: false });
      const deep = new THREE.Mesh(new THREE.PlaneGeometry(26, 11), deepMat);
      deep.position.set(0, WY - 5.4, -6.0);
      deep.renderOrder = 0;
      scene.add(deep);

      const rippleGeo = new THREE.RingGeometry(0.92, 1.0, 48);
      rippleGeo.rotateX(-Math.PI / 2);
      const ripples = [];
      for (let i = 0; i < 3; i++) {
        const mat = new THREE.MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0, depthWrite: false });
        const mesh = new THREE.Mesh(rippleGeo, mat);
        mesh.position.y = WY + 0.02;
        mesh.renderOrder = 3;
        scene.add(mesh);
        ripples.push({ mesh, p: i / 3 });
      }

      const bubbleGeo = new THREE.SphereGeometry(0.05, 10, 10);
      const bubbleMat = new THREE.MeshToonMaterial({ color: 0xe6f2ff, gradientMap, transparent: true, opacity: 0.85, depthWrite: false });
      const bubbles = [];
      for (let i = 0; i < 16; i++) {
        const mesh = new THREE.Mesh(bubbleGeo, bubbleMat);
        mesh.renderOrder = 1;
        scene.add(mesh);
        bubbles.push({
          mesh,
          x: (Math.random() * 2 - 1) * 3.4,
          z: (Math.random() * 2 - 1) * 1.1,
          y: -1.7 + Math.random() * 1.0,
          speed: 0.25 + Math.random() * 0.3,
          phase: Math.random() * Math.PI * 2,
          r: 0.6 + Math.random() * 0.8
        });
      }

      // Cursor tracking — defaults to a bottom-left gaze until the cursor moves
      let mx = -0.8, my = 0.7;
      this._onMove = (ev) => {
        const r = this.getBoundingClientRect();
        if (r.width === 0) return;
        mx = Math.max(-1, Math.min(1, ((ev.clientX - r.left) / r.width) * 2 - 1));
        my = Math.max(-1, Math.min(1, ((ev.clientY - r.top) / r.height) * 2 - 1));
      };
      window.addEventListener('pointermove', this._onMove);

      // Resize — pull the camera back on narrow panels so the span fits
      const resize = () => {
        const w = this.clientWidth || 300;
        const hgt = this.clientHeight || 300;
        renderer.setSize(w, hgt, false);
        camera.aspect = w / hgt;
        camera.position.z = 6.4 / Math.min(camera.aspect / 1.35, 1);
        xOff = Math.min(2.4, camera.position.z * Math.tan(35 * Math.PI / 360) * camera.aspect * 0.44);
        camera.updateProjectionMatrix();
      };
      resize();
      this._ro = new ResizeObserver(resize);
      this._ro.observe(this);

      // Animate
      let t = 0, last = performance.now();
      const loop = (now) => {
        this._raf = requestAnimationFrame(loop);
        const dt = Math.min((now - last) / 1000, 0.05);
        last = now;
        t += dt;

        // swim: gentle bob half-submerged + lateral sway
        arc.position.y = Math.sin(t * 1.2) * 0.06 + 0.02;
        arc.position.x = xOff + Math.sin(t * 0.35) * 0.3;
        arc.rotation.z = Math.sin(t * 0.35 + 1.2) * 0.05;
        spinVel *= Math.exp(-2.4 * dt);
        spinOff += spinVel * dt;
        squashImp *= Math.exp(-5 * dt);
        baseRot += ((mx * 0.55 + Math.sin(t * 0.4) * 0.15) - baseRot) * 0.06;
        arc.rotation.y = baseRot + spinOff;
        arc.rotation.x += ((my * 0.18) - arc.rotation.x) * 0.06;

        for (const tc of tentacles) {
          tc.grp.rotation.y = tc.angle + Math.sin(t * 2.4 + tc.phase) * 0.09;
          tc.grp.rotation.z = Math.sin(t * 2.8 + tc.phase) * 0.05;
        }
        const s = 1 + Math.sin(t * 1.4 + 0.6) * 0.012;
        headGrp.scale.set(s * (1 + squashImp * 0.7), (2 - s) * (1 - squashImp), s);

        // water waves
        const pa = waterGeo.attributes.position;
        for (let i = 0; i < pa.count; i++) {
          const i3 = i * 3;
          const x = waterBase[i3], z = waterBase[i3 + 2];
          pa.array[i3 + 1] = 0.05 * Math.sin(x * 1.9 + t * 1.5) + 0.045 * Math.cos(z * 2.4 + t * 1.1) + 0.02 * Math.sin((x + z) * 3.1 + t * 2.2);
        }
        pa.needsUpdate = true;
        waterGeo.computeVertexNormals();

        // ripples around arc
        if (splashQueued) {
          let oldest = ripples[0];
          for (const r of ripples) if (r.p > oldest.p) oldest = r;
          oldest.p = 0;
          oldest.mesh.position.x = arc.position.x;
          splashQueued = false;
        }
        for (const r of ripples) {
          r.p += dt / 2.6;
          if (r.p >= 1) { r.p -= 1; r.mesh.position.x = arc.position.x; }
          const sc = 0.5 + r.p * 2.1;
          r.mesh.scale.set(sc, 1, sc);
          r.mesh.material.opacity = (1 - r.p) * 0.5;
        }

        // bubbles rising
        for (const b of bubbles) {
          b.y += b.speed * dt;
          if (b.y > WY - 0.06) {
            b.y = -1.75 - Math.random() * 0.25;
            b.x = (Math.random() * 2 - 1) * 3.4;
            b.z = (Math.random() * 2 - 1) * 1.1;
          }
          b.mesh.position.set(b.x + Math.sin(t * 2 + b.phase) * 0.05, b.y, b.z);
          b.mesh.scale.setScalar(b.r);
        }

        renderer.render(scene, camera);
      };
      this._raf = requestAnimationFrame(loop);
    }
  }
  if (!customElements.get('arc-3d')) customElements.define('arc-3d', arc3D);
})();
